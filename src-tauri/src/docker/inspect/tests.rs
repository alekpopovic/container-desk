use super::*;
use crate::contract_tests::scope;
use serde_json::{Value, json};
fn fixture() -> Value {
    json!([{
        "Id":"a".repeat(64),"Name":"/stopped-δ","Image":format!("sha256:{}","b".repeat(64)),"Created":"2026-01-01T10:00:00Z","RestartCount":4,
        "Config":{"Image":"sample:v1","Env":["TOKEN=synthetic-inspect-secret","WITH_EQUALS=a=b","EMPTY="],"Labels":{"innocent":"synthetic-label-secret","password":"synthetic-password"}},
        "State":{"Status":"exited","ExitCode":137,"StartedAt":"2026-01-01T10:00:01Z","FinishedAt":"2026-01-01T11:00:00Z","Health":null,"Error":"synthetic-untrusted-error"},
        "HostConfig":{"Memory":9007199254740993i64,"MemorySwap":-1,"NanoCpus":2000000000i64,"PidsLimit":-1,"ReadonlyRootfs":true,"RestartPolicy":{"Name":"on-failure","MaximumRetryCount":3}},
        "Mounts":[{"Type":"bind","Source":"/home/rootless/data","Destination":"/data","RW":false}],
        "NetworkSettings":{"Networks":{"first":{"NetworkID":"n1","IPAddress":"10.1.0.2","GlobalIPv6Address":"fd00::2","Aliases":["alias"]},"second":null},"Ports":{"80/tcp":[{"HostIp":"::","HostPort":"8080"}],"53/udp":null}},
        "LogPath":"synthetic-private-path","Unknown":"synthetic-unknown-secret"
    }])
}
fn parsed(value: Value, reveal: bool) -> Result<ContainerDetail, AppError> {
    parse(
        &scope(),
        &ContainerId("a".repeat(64)),
        reveal,
        &serde_json::to_vec(&value).unwrap(),
    )
}
#[test]
fn stopped_rootless_paths_no_health_and_multiple_networks_are_exact_and_masked() {
    let detail = parsed(fixture(), false).unwrap();
    let wire = serde_json::to_string(&detail).unwrap();
    for secret in [
        "synthetic-inspect-secret",
        "synthetic-label-secret",
        "synthetic-password",
        "synthetic-untrusted-error",
        "synthetic-private-path",
        "synthetic-unknown-secret",
        "a=b",
    ] {
        assert!(!wire.contains(secret));
        assert!(!format!("{detail:?}").contains(secret));
    }
    assert_eq!(detail.summary.name, "stopped-δ");
    assert_eq!(detail.summary.health, None);
    assert_eq!(detail.exit_code, Some(137));
    assert_eq!(detail.restart_count, Some(4));
    assert_eq!(detail.restart_maximum_retry_count, Some(3));
    assert_eq!(
        detail.resources.memory_bytes.as_deref(),
        Some("9007199254740993")
    );
    assert_eq!(detail.resources.memory_swap_bytes.as_deref(), Some("-1"));
    assert_eq!(
        detail.mounts[0].source.as_deref(),
        Some("/home/rootless/data")
    );
    assert_eq!(detail.networks.len(), 2);
    assert_eq!(detail.networks[0].ipv6.as_deref(), Some("fd00::2"));
    assert_eq!(detail.summary.ports.len(), 2);
    let revealed = parsed(fixture(), true).unwrap();
    assert_eq!(revealed.environment[1].value.as_deref(), Some("a=b"));
    assert_eq!(revealed.environment[2].value.as_deref(), Some(""));
    assert!(!format!("{revealed:?}").contains("synthetic-inspect-secret"));
    assert!(
        parsed(fixture(), false)
            .unwrap()
            .environment
            .iter()
            .all(|v| v.value.is_none())
    );
}
#[test]
fn null_missing_unknown_and_deleted_are_not_invented() {
    let minimal = json!([{"Id":"a".repeat(64),"Config":null,"State":null,"HostConfig":null,"Mounts":null,"NetworkSettings":null}]);
    let detail = parsed(minimal, false).unwrap();
    assert!(detail.networks.is_empty());
    assert!(detail.mounts.is_empty());
    assert_eq!(detail.exit_code, None);
    assert_eq!(detail.resources.memory_bytes, None);
    assert_eq!(detail.summary.state, "unknown");
    assert_eq!(
        parsed(json!([]), false).unwrap_err().code,
        ErrorCode::ContainerNotFound
    );
    let mut value = fixture();
    value[0]["State"]["StartedAt"] = json!("0001-01-01T00:00:00Z");
    value[0]["State"]["Health"] = json!({"Status":"unhealthy","Log":[{"Output":"health-secret"}]});
    let d = parsed(value, false).unwrap();
    assert_eq!(d.started_at, None);
    assert_eq!(d.summary.health.as_deref(), Some("unhealthy"));
    assert!(!serde_json::to_string(&d).unwrap().contains("health-secret"));
}
#[test]
fn hostile_inspect_payloads_and_large_arrays_fail_closed_without_diagnostic_leakage() {
    for value in [
        json!({}),
        json!([{}, {}]),
        json!([{"Id":"b".repeat(64)}]),
        json!([{"Id":"a".repeat(64),"Config":{"Env":["synthetic-no-separator-secret"]}}]),
        json!([{"Id":"a".repeat(64),"State":{"ExitCode":"synthetic-secret"}}]),
    ] {
        let error = parsed(value, false).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidResponse);
        assert!(!serde_json::to_string(&error).unwrap().contains("synthetic"));
    }
    let mut value = fixture();
    value[0]["Config"]["Env"] = json!(vec!["K=x"; 1025]);
    assert_eq!(
        parsed(value, false).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        parse(
            &scope(),
            &ContainerId("a".repeat(64)),
            false,
            &vec![b' '; MAX_BYTES + 1]
        )
        .unwrap_err()
        .code,
        ErrorCode::ResourceLimit
    );
}
#[test]
fn failed_empty_output_is_not_a_valid_inspect_or_raw_error() {
    use std::os::unix::process::ExitStatusExt;
    for (code, stderr, expected) in [
        (
            1,
            format!(
                "Error response from daemon: No such container: {}\n",
                "a".repeat(64)
            ),
            ErrorCode::ContainerNotFound,
        ),
        (
            1,
            format!("Error: No such container: {}\n", "a".repeat(64)),
            ErrorCode::ContainerNotFound,
        ),
        (
            1,
            "synthetic-private-stderr".into(),
            ErrorCode::TransportUnavailable,
        ),
        (
            255,
            "synthetic-private-stderr".into(),
            ErrorCode::Disconnected,
        ),
    ] {
        let error = decode(
            &scope(),
            &ContainerId("a".repeat(64)),
            false,
            Captured {
                status: std::process::ExitStatus::from_raw(code << 8),
                stdout: vec![],
                stderr: stderr.into_bytes(),
            },
        )
        .unwrap_err();
        assert_eq!(error.code, expected);
        assert!(!format!("{error:?}").contains("synthetic"));
    }
}

#[test]
fn health_configuration_oom_and_exposure_are_distinct_from_state_and_bindings() {
    let mut value = fixture();
    value[0]["Config"]["ExposedPorts"] = json!({"80/tcp":{},"53/udp":{}});
    value[0]["Config"]["Healthcheck"] =
        json!({"Test":["CMD-SHELL","echo synthetic-health-secret"]});
    value[0]["State"]["Status"] = json!("running");
    value[0]["State"]["OOMKilled"] = json!(true);
    value[0]["State"]["Health"] = json!({"Status":"unhealthy"});
    value[0]["NetworkSettings"]["Ports"]["80/tcp"] = json!([{"HostIp":"0.0.0.0","HostPort":"8080"},{"HostIp":"::","HostPort":"8080"},{"HostIp":"127.0.0.1","HostPort":"18080"}]);
    let detail = parsed(value.clone(), false).unwrap();
    assert_eq!(detail.healthcheck_configured, Some(true));
    assert_eq!(detail.oom_killed, Some(true));
    assert_eq!(detail.summary.state, "running");
    assert_eq!(detail.summary.health.as_deref(), Some("unhealthy"));
    assert_eq!(detail.exposed_ports.len(), 2);
    assert_eq!(
        detail
            .summary
            .ports
            .iter()
            .filter(|p| p.public_port.is_some())
            .count(),
        3
    );
    assert!(
        !serde_json::to_string(&detail)
            .unwrap()
            .contains("synthetic-health-secret")
    );
    for (check, expected) in [
        (serde_json::Value::Null, Some(false)),
        (json!({"Test":["NONE"]}), Some(false)),
        (json!({"Test":[]}), None),
    ] {
        value[0]["Config"]["Healthcheck"] = check;
        assert_eq!(
            parsed(value.clone(), false).unwrap().healthcheck_configured,
            expected
        );
    }
    value[0]["Config"] = serde_json::Value::Null;
    assert_eq!(parsed(value, false).unwrap().healthcheck_configured, None);
}
