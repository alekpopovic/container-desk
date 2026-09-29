use super::*;
use std::{fs, io, os::unix::fs::PermissionsExt, path::Path, time::Instant};
struct Lab(PathBuf, String);
impl Lab {
    fn new(scenario: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "containerdesk-runner-{}-{}",
            std::process::id(),
            crate::test_directory_suffix()
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path, scenario.into())
    }
    fn executable(&self) -> String {
        "/bin/sh".into()
    }
    fn arguments(&self) -> Vec<OsString> {
        vec![
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/process/child.sh")
                .into_os_string(),
            OsString::from(&self.1),
            self.0.as_os_str().into(),
        ]
    }
    async fn pid(&self) -> libc::pid_t {
        let start = Instant::now();
        loop {
            if let Ok(value) = fs::read_to_string(self.0.join("pid"))
                && let Ok(pid) = value.trim().parse()
            {
                return pid;
            }
            assert!(start.elapsed() < Duration::from_secs(2));
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn reaped(pid: libc::pid_t) {
    // SAFETY: read-only process existence check for the disposable test child's recorded PID.
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}
#[tokio::test]
async fn noisy_stderr_does_not_block_partial_stdout_or_hide_nonzero_exit() {
    let lab = Lab::new("noisy");
    let runner = Runner::default();
    let result = runner
        .start(&lab.executable(), lab.arguments(), Limits::default())
        .unwrap()
        .wait()
        .await
        .unwrap();
    assert_eq!(result.status.code(), Some(7));
    assert_eq!(result.stdout, b"partialtail");
    assert_eq!(result.stderr.len(), 128000);
    assert!(!format!("{result:?}").contains("partialtail"));
    reaped(lab.pid().await);
}
#[tokio::test]
async fn cancellation_and_dropped_waiter_reap_before_releasing_capacity() {
    let lab = Lab::new("sleep");
    let runner = Runner {
        shutdown: tokio::sync::watch::channel(false).0,
        slots: Arc::new(Semaphore::new(1)),
    };
    let mut job = runner
        .start(&lab.executable(), lab.arguments(), Limits::default())
        .unwrap();
    let pid = lab.pid().await;
    assert!(matches!(
        runner.start(&lab.executable(), lab.arguments(), Limits::default()),
        Err(RunError::Busy)
    ));
    job.cancel();
    assert_eq!(job.wait().await.unwrap_err(), RunError::Cancelled);
    reaped(pid);
    fs::remove_file(lab.0.join("pid")).unwrap();
    let job = runner
        .start(&lab.executable(), lab.arguments(), Limits::default())
        .unwrap();
    let pid = lab.pid().await;
    let waiter = tokio::spawn(job.wait());
    waiter.abort();
    let _ = waiter.await;
    let start = Instant::now();
    while runner.slots.available_permits() == 0 {
        assert!(start.elapsed() < Duration::from_secs(2));
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    reaped(pid);
}
#[tokio::test]
async fn timeout_limits_and_empty_success_remain_distinct() {
    let runner = Runner::default();
    let lab = Lab::new("sleep");
    let job = runner
        .start(
            &lab.executable(),
            lab.arguments(),
            Limits {
                deadline: Duration::from_millis(300),
                ..Limits::default()
            },
        )
        .unwrap();
    let pid = lab.pid().await;
    assert_eq!(job.wait().await.unwrap_err(), RunError::TimedOut);
    reaped(pid);
    for (script, stream) in [
        ("stdout_limit", Stream::Stdout),
        ("stderr_limit", Stream::Stderr),
    ] {
        let noisy = Lab::new(script);
        let result = runner
            .start(
                &noisy.executable(),
                noisy.arguments(),
                Limits {
                    stdout_bytes: 1024,
                    stderr_bytes: 1024,
                    ..Limits::default()
                },
            )
            .unwrap()
            .wait()
            .await;
        assert_eq!(result.unwrap_err(), RunError::OutputLimit(stream));
        reaped(noisy.pid().await);
    }
    let empty = Lab::new("empty");
    let result = runner
        .start(&empty.executable(), empty.arguments(), Limits::default())
        .unwrap()
        .wait()
        .await
        .unwrap();
    assert!(result.status.success());
    assert!(result.stdout.is_empty() && result.stderr.is_empty());
    reaped(empty.pid().await);
}
#[tokio::test]
async fn cancelled_before_dispatch_does_not_launch_and_bad_bounds_are_rejected() {
    let lab = Lab::new("exit");
    let runner = Runner::default();
    let mut job = runner
        .start(&lab.executable(), lab.arguments(), Limits::default())
        .unwrap();
    job.cancel();
    assert_eq!(job.wait().await.unwrap_err(), RunError::Cancelled);
    assert!(!lab.0.join("pid").exists());
    assert!(matches!(
        runner.start(
            &lab.executable(),
            lab.arguments(),
            Limits {
                deadline: Duration::ZERO,
                ..Limits::default()
            }
        ),
        Err(RunError::InvalidInput)
    ));
    assert!(matches!(
        runner.start(
            &lab.executable(),
            vec![OsString::from("x\0y")],
            Limits::default()
        ),
        Err(RunError::InvalidInput)
    ));
}
#[test]
fn native_ssh_accepts_structured_bounds_and_trust_options_without_connecting() {
    let selected = SshSelection {
        alias: "fixture-target".into(),
        config_path: "/dev/null".into(),
        use_default_config: false,
    };
    let args = structured_arguments(&selected).unwrap();
    let result = std::process::Command::new("/usr/bin/ssh")
        .arg("-G")
        .args(args)
        .output()
        .unwrap();
    assert!(result.status.success());
    let stdout = String::from_utf8(result.stdout).unwrap();
    for line in [
        "connecttimeout 10",
        "serveraliveinterval 15",
        "serveralivecountmax 2",
        "batchmode yes",
        "forwardagent no",
        "requesttty false",
        "controlmaster false",
        "updatehostkeys false",
        "stricthostkeychecking true",
    ] {
        assert!(stdout.lines().any(|v| v == line), "missing {line}");
    }
}

#[tokio::test]
async fn closed_session_rejects_a_late_job_before_dispatch() {
    let lab = Lab::new("exit");
    let runner = Runner::default();
    let (sender, receiver) = tokio::sync::watch::channel(false);
    drop(sender);
    let job = runner
        .start_for_session(
            &lab.executable(),
            lab.arguments(),
            Limits::default(),
            Box::new(()),
            Some(receiver),
        )
        .unwrap();
    assert_eq!(job.wait().await.unwrap_err(), RunError::Cancelled);
    assert!(!lab.0.join("pid").exists());
}

#[tokio::test]
async fn shutdown_cancels_owned_jobs_reaps_and_does_not_touch_another_runner() {
    let runner = Runner::default();
    let other = Runner::default();
    let owned = Lab::new("sleep");
    let unrelated = Lab::new("sleep");
    let job = runner
        .start(&owned.executable(), owned.arguments(), Limits::default())
        .unwrap();
    let mut survivor = other
        .start(
            &unrelated.executable(),
            unrelated.arguments(),
            Limits::default(),
        )
        .unwrap();
    let pid = owned.pid().await;
    let other_pid = unrelated.pid().await;
    let stream_lab = Lab::new("sleep");
    let stream = runner
        .start_stream(
            &stream_lab.executable(),
            stream_lab.arguments(),
            Arc::new(|_, _, _| {}),
            Box::new(()),
            None,
        )
        .unwrap();
    let stream_pid = stream_lab.pid().await;
    runner.cancel_all();
    tokio::time::timeout(Duration::from_secs(2), runner.wait_idle())
        .await
        .unwrap();
    assert_eq!(job.wait().await.unwrap_err(), RunError::Cancelled);
    reaped(pid);
    assert_eq!(stream.wait().await.unwrap_err(), RunError::Cancelled);
    reaped(stream_pid);
    assert_eq!(unsafe { libc::kill(other_pid, 0) }, 0);
    assert_eq!(
        runner
            .start(&owned.executable(), owned.arguments(), Limits::default())
            .unwrap()
            .wait()
            .await
            .unwrap_err(),
        RunError::Cancelled
    );
    survivor.cancel();
    assert_eq!(survivor.wait().await.unwrap_err(), RunError::Cancelled);
    reaped(other_pid);
}
