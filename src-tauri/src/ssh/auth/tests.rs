use super::*;
use std::os::unix::fs::MetadataExt;
#[test]
fn diagnostics_exclude_banners_paths_and_secrets_and_prioritize_trust() {
    let (status, message) = classify(
        b"secret banner\nREMOTE HOST IDENTIFICATION HAS CHANGED!\nPermission denied\n/private/key",
    );
    assert_eq!(status, SshAccessStatus::ChangedHostKey);
    assert_eq!(message, Some("REMOTE HOST IDENTIFICATION HAS CHANGED!"));
    assert_eq!(
        classify(b"server secret"),
        (SshAccessStatus::ConnectionFailed, None)
    );
}
#[test]
fn policy_references_source_without_copying_and_cleans_only_owned_entries() {
    let root = Path::new("/tmp").join(format!(
        "containerdesk-auth-test-{}",
        crate::test_directory_suffix()
    ));
    fs::create_dir(&root).unwrap();
    let source = root.join("source Željko-$%-config");
    fs::write(&source, b"Host fixture\n HostName 192.0.2.1\n").unwrap();
    let selected = SshSelection {
        alias: "fixture".into(),
        config_path: source.to_str().unwrap().into(),
        use_default_config: false,
    };
    let policy = PolicyConfig::create(&selected).unwrap();
    let directory = policy.runtime.path().to_path_buf();
    assert_eq!(fs::metadata(&directory).unwrap().mode() & 0o777, 0o700);
    assert_eq!(
        fs::metadata(&policy.selection.config_path).unwrap().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read_link(directory.join("user.conf")).unwrap(), source);
    assert!(
        !fs::read_to_string(&policy.selection.config_path)
            .unwrap()
            .contains("HostName")
    );
    drop(policy);
    assert!(!directory.exists());
    assert!(source.exists());
    fs::remove_file(source).unwrap();
    assert!(PolicyConfig::create(&selected).is_err());
    let default = SshSelection {
        use_default_config: true,
        ..selected
    };
    let policy = PolicyConfig::create(&default).unwrap();
    assert!(
        fs::read_to_string(&policy.selection.config_path)
            .unwrap()
            .contains("Include /etc/ssh/ssh_config")
    );
    drop(policy);
    fs::remove_dir(root).unwrap();
}

#[test]
fn overlay_enforces_both_hops_and_retains_native_identity_and_include_resolution() {
    let root = Path::new("/tmp").join(format!(
        "containerdesk-auth-options-{}",
        crate::test_directory_suffix()
    ));
    fs::create_dir(&root).unwrap();
    let source = root.join("source.conf");
    fs::write(&source, "Host *\n BatchMode no\n ForwardAgent yes\n StrictHostKeyChecking no\n PasswordAuthentication yes\n KbdInteractiveAuthentication yes\n UpdateHostKeys yes\nHost jump\n HostName 192.0.2.2\n IdentityFile /fixture/jump-key\nHost target\n HostName 192.0.2.3\n ProxyJump jump\n IdentityFile /fixture/target-key\n").unwrap();
    for alias in ["jump", "target"] {
        let selected = SshSelection {
            alias: alias.into(),
            config_path: source.to_str().unwrap().into(),
            use_default_config: false,
        };
        let policy = PolicyConfig::create(&selected).unwrap();
        let result = std::process::Command::new("/usr/bin/ssh")
            .args(super::super::resolver::arguments(&policy.selection).unwrap())
            .output()
            .unwrap();
        assert!(result.status.success());
        let text = String::from_utf8(result.stdout).unwrap();
        for line in [
            "batchmode yes",
            "forwardagent no",
            "stricthostkeychecking true",
            "passwordauthentication no",
            "kbdinteractiveauthentication no",
            "updatehostkeys false",
        ] {
            assert!(
                text.lines().any(|value| value == line),
                "missing {line} for {alias}"
            );
        }
        assert!(text.contains(&format!("identityfile /fixture/{alias}-key")));
    }
    fs::remove_file(source).unwrap();
    fs::remove_dir(root).unwrap();
}

#[tokio::test]
#[ignore = "requires explicitly provisioned disposable SSH lab; run tests/lab/ssh_auth.py"]
async fn disposable_lab_authentication_matrix() {
    let manifest =
        std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest required");
    let lab: serde_json::Value = serde_json::from_slice(&fs::read(manifest).unwrap()).unwrap();
    let runner = Runner::default();
    for case in lab["cases"].as_array().unwrap() {
        let alias = case["alias"].as_str().unwrap();
        let expected: SshAccessStatus = serde_json::from_value(case["expected"].clone()).unwrap();
        let start = std::time::Instant::now();
        let report = probe(
            &runner,
            "/usr/bin/ssh",
            SshSelection {
                alias: alias.into(),
                config_path: lab["config"].as_str().unwrap().into(),
                use_default_config: false,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            report.status, expected,
            "case {alias}; sanitized report: {report:?}"
        );
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "local failure/success must be prompt: {alias}"
        );
        println!(
            "lab {alias}: {:?}, {} ms",
            report.status,
            start.elapsed().as_millis()
        );
    }
    assert!(
        !Path::new(lab["askpassMarker"].as_str().unwrap()).exists(),
        "no hidden askpass invocation"
    );
}
