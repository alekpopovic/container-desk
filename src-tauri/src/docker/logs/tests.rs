use super::*;
use std::os::unix::process::ExitStatusExt;
fn request() -> ContainerLogsRequest {
    ContainerLogsRequest {
        scope: crate::contract_tests::scope(),
        container_id: ContainerId("a".repeat(64)),
        tail: 100,
        timeout_seconds: 30,
        since: None,
        until: None,
    }
}
fn output(stdout: Vec<u8>, stderr: Vec<u8>, code: i32) -> Captured {
    Captured {
        status: std::process::ExitStatus::from_raw(code << 8),
        stdout,
        stderr,
    }
}
#[test]
fn mixed_application_channels_are_timestamp_ordered_and_stderr_stays_ambiguous() {
    let snapshot = decode(
        &request(),
        output(
            b"2026-01-01T10:00:00.000000001Z out1\n2026-01-01T10:00:00.000000003Z out2\n".to_vec(),
            b"2026-01-01T10:00:00.000000002Z err1\npossible transport notice\n".to_vec(),
            0,
        ),
    )
    .unwrap();
    assert_eq!(
        snapshot
            .records
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>(),
        ["out1", "err1", "out2", "possible transport notice"]
    );
    assert_eq!(snapshot.records[1].channel, LogChannel::StderrAmbiguous);
    assert!(snapshot.stderr_ambiguous);
    assert!(!snapshot.truncated);
    assert!(!format!("{snapshot:?}").contains("out1"));
    assert!(!format!("{snapshot:?}").contains("transport notice"));
}
#[test]
fn empty_success_differs_from_daemon_driver_and_ssh_failure_without_raw_diagnostics() {
    assert!(
        decode(&request(), output(vec![], vec![], 0))
            .unwrap()
            .records
            .is_empty()
    );
    for (code, message, expected) in [
        (
            1,
            "Error response from daemon: configured logging driver does not support reading",
            ErrorCode::LogDriverUnsupported,
        ),
        (
            1,
            "synthetic-daemon-secret",
            ErrorCode::TransportUnavailable,
        ),
        (255, "synthetic-ssh-secret", ErrorCode::Disconnected),
    ] {
        let error = decode(
            &request(),
            output(
                b"synthetic-partial-secret".to_vec(),
                message.as_bytes().to_vec(),
                code,
            ),
        )
        .unwrap_err();
        assert_eq!(error.code, expected);
        assert!(!format!("{error:?}").contains("synthetic"));
    }
}
#[test]
fn oversized_and_invalid_utf8_lines_are_marked_and_retention_stays_bounded() {
    let snapshot = decode(
        &request(),
        output(
            vec![b'x'; MAX_RECORD_BYTES + 100],
            vec![255; MAX_RECORD_BYTES],
            0,
        ),
    )
    .unwrap();
    assert!(snapshot.truncated);
    assert_eq!(snapshot.records.len(), 2);
    assert!(
        snapshot
            .records
            .iter()
            .all(|r| r.text.len() <= MAX_RECORD_BYTES)
    );
    assert!(snapshot.records[1].invalid_utf8);
    let snapshot = decode(
        &request(),
        output(b"a\n".repeat(MAX_RECORDS), b"b\n".repeat(MAX_RECORDS), 0),
    )
    .unwrap();
    assert_eq!(snapshot.records.len(), MAX_RECORDS);
    assert_eq!(snapshot.dropped_records, MAX_RECORDS as u32);
    assert!(snapshot.truncated);
    assert_eq!(
        decode(
            &request(),
            output(vec![b'\n'; MAX_RETAINED_BYTES + 1], vec![], 0)
        )
        .unwrap_err()
        .code,
        ErrorCode::ResourceLimit
    );
}
#[test]
fn log_ranges_are_numeric_bounded_and_cannot_be_shell_arguments() {
    for invalid in [
        "",
        "-1",
        "now",
        "$(id)",
        "1;id",
        "1\n",
        "1.",
        "1.1234567890",
        "253402300800",
        "1e3",
        "--follow",
    ] {
        assert!(registry::validate_log_range(Some(invalid), None).is_err());
    }
    assert!(registry::validate_log_range(Some("1.2"), Some("1.19")).is_err());
    assert!(registry::validate_log_range(Some("1.2"), Some("1.20")).is_ok());
    let plan = registry::read(&ReadOperation::ContainerLogs {
        container_id: request().container_id,
        tail: 1,
        timeout_seconds: 1,
        since: Some("1.000000001".into()),
        until: Some("2".into()),
    })
    .unwrap();
    assert!(
        plan.args()
            .windows(2)
            .any(|a| a == ["--since", "1.000000001"])
    );
    assert!(
        !plan
            .args()
            .iter()
            .any(|a| a == "--details" || a == "--follow")
    );
}

#[test]
fn combined_channel_retention_respects_shared_byte_budget() {
    let line = [vec![b'x'; 100_000], vec![b'\n']].concat();
    let snapshot = decode(&request(), output(line.repeat(50), line.repeat(50), 0)).unwrap();
    assert!(snapshot.truncated);
    assert!(snapshot.dropped_records > 0);
    assert!(snapshot.records.iter().map(size).sum::<usize>() <= MAX_RETAINED_BYTES);
    assert_eq!(
        snapshot.records.len() as u32 + snapshot.dropped_records,
        100
    );
}
