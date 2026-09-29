use super::*;
use crate::ssh::runner::Runner;
use std::sync::atomic::AtomicUsize;
fn request() -> FollowLogsRequest {
    FollowLogsRequest {
        scope: crate::contract_tests::scope(),
        container_id: ContainerId("a".repeat(64)),
        tail: 100,
        since: Some("1.000000001".into()),
    }
}
fn row(text: String) -> LogRecord {
    LogRecord {
        text,
        timestamp: None,
        channel: LogChannel::Stdout,
        truncated: false,
        invalid_utf8: false,
    }
}
#[test]
fn shared_retention_and_batches_bound_both_line_count_and_bytes() {
    let mut queue = LogQueue::default();
    for _ in 0..25_000 {
        queue.push(row("x".into()));
    }
    assert_eq!(queue.records.len(), MAX_RECORDS);
    assert_eq!(queue.dropped, 5000);
    let (batch, dropped) = queue.batch();
    assert_eq!(batch.len(), BATCH_LINES);
    assert_eq!(dropped, 5000);
    for _ in 0..100 {
        queue.push(row("x".repeat(256 * 1024)));
    }
    assert!(queue.bytes <= MAX_RETAINED_BYTES);
    assert!(queue.records.len() <= 32);
    assert!(queue.dropped > 0);
    let (batch, _) = queue.batch();
    assert_eq!(batch.len(), 1);
    assert!(batch.iter().map(size).sum::<usize>() <= BATCH_BYTES);
    let plan = crate::policy::registry::read(&crate::policy::registry::ReadOperation::FollowLogs {
        container_id: request().container_id,
        tail: 100,
        since: request().since,
    })
    .unwrap();
    assert!(plan.args().iter().any(|a| a == "--follow"));
    assert!(!plan.args().iter().any(|a| a == "--details"));
}
#[tokio::test]
async fn slow_consumption_drops_data_without_extra_ipc_and_cancel_reaps_heavy_child() {
    let root = std::env::temp_dir().join(format!(
        "containerdesk-live-unit-{}",
        crate::test_directory_suffix()
    ));
    std::fs::create_dir(&root).unwrap();
    let pid_file = root.join("pid");
    let queue = Arc::new(Mutex::new(LogQueue::default()));
    let runner = Runner::default();
    // Test fixture only; application callers cannot submit shell commands.
    let script = format!(
        "echo $$ > '{}'; exec /usr/bin/yes synthetic-live-unit",
        pid_file.display()
    );
    let job = runner
        .start_stream(
            "/bin/sh",
            vec!["-c".into(), script.into()],
            line_sink(queue.clone()),
            Box::new(()),
            None,
        )
        .unwrap();
    let subscriptions = Subscriptions::default();
    let slots = Arc::new(Semaphore::new(4));
    let (sender, mut receiver) = mpsc::channel(2);
    let count = Arc::new(AtomicUsize::new(0));
    let sent = count.clone();
    let response = subscriptions
        .start(
            request(),
            job,
            queue.clone(),
            Arc::new(move |batch| {
                sent.fetch_add(1, Ordering::SeqCst);
                sender.try_send(batch).map_err(|_| ())
            }),
            Arc::new(|| Ok(())),
            (
                slots.clone().try_acquire_owned().unwrap(),
                subscriptions.reserve().unwrap(),
            ),
        )
        .unwrap();
    let ack = |sequence| AckLogsRequest {
        scope: response.scope.clone(),
        subscription_id: response.subscription_id.clone(),
        sequence,
    };
    subscriptions.ack(&ack(0)).unwrap();
    let batch = tokio::time::timeout(Duration::from_secs(3), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(
        batch.gap,
        "timestamp resume must visibly mark possible gaps"
    );
    assert_eq!(batch.sequence, 1);
    // Renderer withholds ACK, while a real child keeps flooding stdout.
    tokio::time::sleep(Duration::from_millis(650)).await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    {
        let q = queue.lock().unwrap();
        println!(
            "Paused heavy reader: {} records, {} retained text bytes, {} dropped; one IPC batch in flight",
            q.records.len(),
            q.bytes,
            q.dropped
        );
        assert!(q.dropped > 0);
        assert!(q.bytes <= MAX_RETAINED_BYTES);
        assert!(q.records.len() <= MAX_RECORDS);
    }
    let mut foreign = ack(1);
    foreign.scope.daemon_id = "foreign".into();
    assert_eq!(
        subscriptions.ack(&foreign).unwrap_err().code,
        ErrorCode::SubscriptionNotFound
    );
    assert_eq!(
        subscriptions.ack(&ack(99)).unwrap_err().code,
        ErrorCode::InvalidResponse
    );
    subscriptions.ack(&ack(1)).unwrap();
    assert_eq!(
        subscriptions.ack(&ack(1)).unwrap_err().code,
        ErrorCode::InvalidResponse
    );
    let next = tokio::time::timeout(Duration::from_secs(3), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(next.dropped_records > 0 && next.gap);
    let pid: i32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    tokio::time::timeout(
        Duration::from_secs(3),
        subscriptions.cancel(&CancelSubscriptionRequest {
            scope: response.scope,
            subscription_id: response.subscription_id,
        }),
    )
    .await
    .unwrap()
    .unwrap();
    // SAFETY: signal 0 only checks existence; no unrelated process is signalled.
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "owned stream leader must already be reaped"
    );
    runner.wait_idle().await;
    assert_eq!(slots.available_permits(), 4);
    assert!(subscriptions.entries.lock().unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
async fn stopped_session_interrupts_silent_and_oversized_streams() {
    let runner = Runner::default();
    for script in [
        "exec /bin/sleep 60",
        "exec /usr/bin/head -c 300000 /dev/zero",
    ] {
        let queue = Arc::new(Mutex::new(LogQueue::default()));
        let (closed, receiver) = watch::channel(false);
        let mut job = runner
            .start_stream(
                "/bin/sh",
                vec!["-c".into(), script.into()],
                line_sink(queue.clone()),
                Box::new(()),
                Some(receiver),
            )
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        if script.contains("head") {
            let result = job.finished().await.unwrap();
            assert!(result.status.success());
            let q = queue.lock().unwrap();
            assert_eq!(q.records.len(), 1);
            assert!(q.records[0].truncated);
            assert!(q.records[0].text.len() <= 256 * 1024);
        } else {
            closed.send_replace(true);
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(2), job.finished())
                    .await
                    .unwrap()
                    .unwrap_err(),
                RunError::Cancelled
            );
        }
    }
    runner.wait_idle().await;
}

#[tokio::test]
async fn abandoned_renderer_expires_and_reaps_without_any_ack() {
    let runner = Runner::default();
    let queue = Arc::new(Mutex::new(LogQueue::default()));
    let job = runner
        .start_stream(
            "/bin/sleep",
            vec!["60".into()],
            line_sink(queue.clone()),
            Box::new(()),
            None,
        )
        .unwrap();
    let subscriptions = Subscriptions::default();
    let slots = Arc::new(Semaphore::new(4));
    subscriptions
        .start(
            request(),
            job,
            queue,
            Arc::new(|_| panic!("unacknowledged renderer cannot receive data")),
            Arc::new(|| Ok(())),
            (
                slots.clone().try_acquire_owned().unwrap(),
                subscriptions.reserve().unwrap(),
            ),
        )
        .unwrap();
    tokio::time::timeout(Duration::from_secs(33), async {
        loop {
            if subscriptions.entries.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    runner.wait_idle().await;
    assert_eq!(slots.available_permits(), 4);
    assert_eq!(subscriptions.slots.available_permits(), 2);
}
