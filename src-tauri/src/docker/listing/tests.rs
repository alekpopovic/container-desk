use super::*;
use crate::contract_tests::scope;
use serde_json::json;
fn record(n: usize) -> serde_json::Value {
    json!({"ID": format!("{n:064x}"), "Names": "web", "Image": "example:latest",
        "State": "running", "Status": "Up 3 minutes (unhealthy)",
        "Ports": "0.0.0.0:8080->80/tcp, [::]:8080->80/tcp, 9000-9009/udp",
        "Labels": "project=lab,secret.token=do-not-export,comma=value,looks=like-another-label",
        "CreatedAt": "2026-09-29 00:00:00 +0000 UTC", "RunningFor": "3 minutes ago",
        "FutureField": {"ignored": true}})
}
#[test]
fn native_cli_fields_are_normalized_without_inventing_inspect_precision() {
    let mut records = vec![];
    for (n, state, status) in [
        (1, "running", "Up 3 minutes (unhealthy)"),
        (2, "exited", "Exited (137) 1 minute ago"),
        (3, "future-state", "Unknown status"),
    ] {
        let mut value = record(n);
        value["State"] = json!(state);
        value["Status"] = json!(status);
        value["Names"] = json!("čćž-服务, linked-name");
        records.push(value.to_string());
    }
    let result = parse(&scope(), (records.join("\r\n") + "\r\n").as_bytes()).unwrap();
    assert_eq!(result.containers.len(), 3);
    for (n, value) in result.containers.iter().enumerate() {
        assert_eq!(value.id.0, format!("{:064x}", n + 1));
        assert_eq!(value.scope, scope());
        assert_eq!(value.name, "čćž-服务");
        assert!(value.health.is_none());
        assert!(value.ports.is_empty());
        assert!(value.compose.is_none());
        let cli = value.cli.as_ref().unwrap();
        assert_eq!(cli.names, ["čćž-服务", "linked-name"]);
        assert!(cli.ports.as_ref().unwrap().contains("9000-9009/udp"));
        assert_eq!(cli.labels_present, Some(true));
    }
    assert_eq!(result.containers[0].status, "Up 3 minutes (unhealthy)");
    assert_eq!(result.containers[1].state, "exited");
    let serialized = serde_json::to_string(&result).unwrap();
    assert!(!serialized.contains("do-not-export"));
    assert!(!serialized.contains("looks=like-another-label"));
}
#[test]
fn missing_optional_values_are_unknown_and_successful_zero_bytes_is_empty() {
    assert!(parse(&scope(), b"").unwrap().containers.is_empty());
    let value = json!({"ID": "a".repeat(64), "Ports": null});
    let result = parse(&scope(), value.to_string().as_bytes()).unwrap();
    let item = &result.containers[0];
    assert_eq!(item.name, "a".repeat(64));
    assert_eq!(item.image, "Unknown");
    assert_eq!(item.state, "unknown");
    assert_eq!(item.cli.as_ref().unwrap().labels_present, None);
    let mut empty = record(2);
    empty["Labels"] = json!("");
    assert_eq!(
        parse(&scope(), empty.to_string().as_bytes())
            .unwrap()
            .containers[0]
            .cli
            .as_ref()
            .unwrap()
            .labels_present,
        Some(false)
    );
}
#[test]
fn banners_wrong_types_short_ids_duplicates_and_blank_records_fail_the_whole_snapshot() {
    let valid = record(1).to_string();
    for bytes in [
        "banner\n".to_owned() + &valid,
        valid.clone() + "\nnot-json",
        "[]".into(),
        "null".into(),
        "{}".into(),
        "\n".into(),
        " \n".into(),
        valid.clone() + "\n\n",
        valid.clone() + "\n" + &valid,
        json!({"ID":"a".repeat(12)}).to_string(),
        json!({"ID":"A".repeat(64)}).to_string(),
        json!({"ID":"a".repeat(64),"Names":123}).to_string(),
    ] {
        let error = parse(&scope(), bytes.as_bytes()).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidResponse);
        assert_eq!(error.scope, Some(scope()));
        assert!(!serde_json::to_string(&error).unwrap().contains("banner"));
    }
}
#[test]
fn response_records_and_fields_have_explicit_limits_and_large_valid_lists_keep_full_ids() {
    assert_eq!(
        parse(&scope(), &vec![b' '; MAX_BYTES + 1])
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        parse(&scope(), &vec![b' '; MAX_RECORD_BYTES + 1])
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    let mut large = record(1);
    large["Labels"] = json!("a".repeat(64 * 1024 + 1));
    assert_eq!(
        parse(&scope(), large.to_string().as_bytes())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    let records: String = (0..MAX_RECORDS)
        .map(|n| format!("{{\"ID\":\"{n:064x}\"}}\n"))
        .collect();
    let parsed = parse(&scope(), records.as_bytes()).unwrap();
    assert_eq!(parsed.containers.len(), MAX_RECORDS);
    assert_eq!(
        parsed.containers[MAX_RECORDS - 1].id.0,
        format!("{:064x}", MAX_RECORDS - 1)
    );
    drop(parsed);
    let oversized = records + &format!("{{\"ID\":\"{MAX_RECORDS:064x}\"}}\n");
    assert_eq!(
        parse(&scope(), oversized.as_bytes()).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn empty_failed_commands_do_not_become_success_or_expose_raw_diagnostics() {
    use std::os::unix::process::ExitStatusExt;
    for (code, expected) in [
        (1, ErrorCode::TransportUnavailable),
        (255, ErrorCode::Disconnected),
    ] {
        let error = decode(
            &scope(),
            crate::ssh::runner::Captured {
                status: std::process::ExitStatus::from_raw(code << 8),
                stdout: vec![],
                stderr: b"secret remote diagnostic".to_vec(),
            },
        )
        .unwrap_err();
        assert_eq!(error.code, expected);
        assert!(!serde_json::to_string(&error).unwrap().contains("secret"));
    }
    assert!(
        decode(
            &scope(),
            crate::ssh::runner::Captured {
                status: std::process::ExitStatus::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            }
        )
        .unwrap()
        .containers
        .is_empty()
    );
}

#[tokio::test]
#[ignore = "requires explicitly seeded disposable real Docker Engine and SSH aliases"]
async fn checkpoint019_real_listing_matches_engine_ids_through_direct_and_proxyjump() {
    assert_eq!(std::env::var("PATH").unwrap(), "/nonexistent");
    let lab: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            std::env::var("CONTAINERDESK_SSH_LAB_MANIFEST").expect("explicit lab manifest"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(lab["realEngine"], true);
    let expected: HashSet<String> =
        serde_json::from_value(lab["expectedContainerIds"].clone()).unwrap();
    assert_eq!(expected.len(), 2);
    for alias in ["direct-known", "via-known"] {
        let mut connection = Connection::new(
            "/usr/bin/ssh",
            SshSelection {
                alias: alias.into(),
                config_path: lab["config"].as_str().unwrap().into(),
                use_default_config: false,
            },
        )
        .unwrap();
        assert_eq!(
            connection.start().await.unwrap().status,
            SshAccessStatus::Verified
        );
        let options = DockerOptions::default();
        let (report, binding) = super::super::probe::run(&connection, &options).await;
        assert_eq!(report.status, DockerProbeStatus::Ready);
        let mut current = scope();
        current.daemon_id = report.daemon_id.unwrap();
        let binding = binding.unwrap();
        let result = read(&connection, &options, &binding, &current)
            .await
            .unwrap();
        let actual: HashSet<_> = result
            .containers
            .iter()
            .map(|item| item.id.0.clone())
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(result.containers.len(), 2);
        assert!(result.containers.iter().all(|item| item.state == "created"
            && item.cli.as_ref().unwrap().labels_present == Some(true)));
        let independent = connection
            .start_fixed(
                crate::ssh::quoting::command(&[
                    "docker".into(),
                    "ps".into(),
                    "--all".into(),
                    "--quiet".into(),
                    "--no-trunc".into(),
                ])
                .unwrap(),
                Limits::default(),
            )
            .unwrap()
            .wait()
            .await
            .unwrap();
        assert!(independent.status.success());
        assert_eq!(
            String::from_utf8(independent.stdout)
                .unwrap()
                .lines()
                .map(String::from)
                .collect::<HashSet<_>>(),
            actual
        );
        let mut wrong = current;
        wrong.daemon_id = "not-this-daemon".into();
        assert_eq!(
            read(&connection, &options, &binding, &wrong)
                .await
                .unwrap_err()
                .code,
            ErrorCode::StaleSession
        );
        connection.close().await;
        println!(
            "PASS real Docker listing {alias}: 2 complete IDs/count match independent native CLI and provisioning; wrong daemon rejected; client PATH=/nonexistent"
        );
    }
}
