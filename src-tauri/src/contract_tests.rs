use crate::domain::*;
use serde_json::json;
use std::path::Path;
use ts_rs::{Config, TS};

pub fn scope() -> SessionScope {
    SessionScope {
        selection: HostSelection {
            host_id: HostId(format!("h_{}", "1".repeat(32))),
            selection_generation: 7,
        },
        session_id: SessionId(format!("s_{}", "2".repeat(32))),
        session_generation: 3,
        daemon_id: "fixture-daemon".into(),
    }
}

fn check_or_update(path: &Path, content: &str) {
    if std::env::var_os("CONTAINERDESK_UPDATE_CONTRACTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    } else {
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            content,
            "Generated contract drift at {}. Run npm run ipc:generate",
            path.display()
        );
    }
}

#[test]
fn generated_contract_is_current() {
    let config = Config::default();
    let mut output =
        "// Generated from src-tauri/src/domain.rs with ts-rs. Do not edit.\n".to_string();
    macro_rules! export { ($($type:ty),* $(,)?) => { $(
        output.push_str("export "); output.push_str(&<$type>::decl(&config).lines().map(str::trim_end).collect::<Vec<_>>().join("\n")); output.push('\n');
    )* }; }
    export!(
        HostId,
        SessionId,
        ContainerId,
        SubscriptionId,
        HostSelection,
        SessionScope,
        ErrorCode,
        AppError,
        ConnectionState,
        HostCapabilities,
        HostSummary,
        ContainerSummary,
        ContainerDetail,
        ListHostsResponse,
        ConnectHostRequest,
        ConnectHostResponse,
        ListContainersRequest,
        ListContainersResponse,
        CancelSubscriptionRequest,
        CancelSubscriptionResponse,
        AppVersion
    );
    check_or_update(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/ipc/generated.ts"),
        &output,
    );
}

#[test]
fn serialized_fixtures_are_current() {
    let summary = ContainerSummary {
        scope: scope(),
        id: ContainerId("a".repeat(64)),
        name: "example".into(),
        image: "example:fixture".into(),
        state: "running".into(),
        status: "Up".into(),
        health: None,
    };
    let detail = ContainerDetail {
        summary: summary.clone(),
        environment_names: vec!["EXAMPLE_TOKEN".into()],
        environment_values_masked: true,
    };
    let fixtures = json!({
        "success": ListContainersResponse { scope: scope(), containers: vec![summary] },
        "detail": detail,
        "error": AppError::new(ErrorCode::SessionNotFound).in_scope(&scope()),
        "connected": ConnectHostResponse { scope: scope(), capabilities: HostCapabilities { docker: true, compose: false, management: false, terminal: false } },
        "hosts": ListHostsResponse { hosts: vec![HostSummary { id: scope().selection.host_id, alias: "fixture".into(), display_name: "Example".into(), group: "Lab".into(), read_only: true, connection_state: ConnectionState::Disconnected }] }
    });
    let content = serde_json::to_string_pretty(&fixtures).unwrap() + "\n";
    assert!(!content.contains("environmentValues\""));
    let response: ListContainersResponse =
        serde_json::from_value(fixtures["success"].clone()).unwrap();
    assert_eq!(response.scope, scope());
    let error: AppError = serde_json::from_value(fixtures["error"].clone()).unwrap();
    assert_eq!(error.code, ErrorCode::SessionNotFound);
    check_or_update(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/ipc.json"),
        &content,
    );
}
