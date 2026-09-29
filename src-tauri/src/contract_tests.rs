use crate::activity::{ActivityOutcome, ActivityRecord};
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
        ResourceLimits,
        ResourceLimitsReport,
        SupportPreview,
        SaveSupportRequest,
        ClearSupportRequest,
        HostId,
        SessionId,
        ContainerId,
        ImageId,
        VolumeName,
        NetworkId,
        ComposeVerificationId,
        ComposeActionOperation,
        ComposeConfiguration,
        VerifyComposeRequest,
        ComposeVerification,
        ComposeActionSpec,
        ComposeMutationRequest,
        ComposeMutationResponse,
        ListNetworksRequest,
        InspectNetworkRequest,
        NetworkSummary,
        ListNetworksResponse,
        NetworkAddress,
        NetworkIpamConfig,
        NetworkAttachment,
        NetworkDetail,
        ListVolumesRequest,
        ListVolumesResponse,
        VolumeSummary,
        InspectVolumeRequest,
        VolumeDetail,
        VolumeMountReference,
        ReferenceObservation,
        ListImagesRequest,
        ListImagesResponse,
        ImageSummary,
        InspectImageRequest,
        ImageDetail,
        ImageContainerReference,
        SubscriptionId,
        HostSelection,
        SessionScope,
        ErrorCode,
        AppError,
        ConnectionState,
        HostCapabilities,
        HostSummary,
        ContainerSummary,
        ContainerListDisplay,
        ContainerDetail,
        ExposedPort,
        DetailValue,
        DetailMount,
        DetailNetwork,
        ResourceConfiguration,
        ListHostsResponse,
        ConnectHostRequest,
        ConnectHostResponse,
        ListComposeRequest,
        ListComposeResponse,
        ComposeProject,
        ComposeInstance,
        ComposeConfigurationStatus,
        ListContainersRequest,
        ListContainersResponse,
        CancelSubscriptionRequest,
        CancelSubscriptionResponse,
        AppVersion,
        Theme,
        SavedHost,
        Preferences,
        StorageNotice,
        PreferencesSnapshot,
        SetThemeRequest,
        SshStatus,
        SshDiagnostic,
        AgentStatus,
        AgentDiagnostic,
        DependencyDiagnostics,
        SetSshExecutableRequest,
        SetSshExecutableResponse,
        IntentId,
        HostAccess,
        SetManagementRequest,
        ManagementState,
        ActivityRecord,
        ActivityOutcome,
        MutationOperation,
        MutationSpec,
        TerminalShell,
        TerminalSpec,
        ConfirmationOperation,
        PrepareConfirmationRequest,
        ConfirmationIntent,
        MutationRequest,
        MutationOutcome,
        MutationTargetOutcome,
        MutationTargetResult,
        CancelMutationRequest,
        CancelMutationResponse,
        MutationResponse,
        TerminalRequest,
        TerminalResponse,
        TerminalHandleRequest,
        TerminalInputRequest,
        TerminalResizeRequest,
        TerminalState,
        TerminalOutput,
        InspectContainerRequest,
        ContainerLogsRequest,
        LogSnapshot,
        ContainerStatsRequest,
        StatsAvailability,
        StatsValues,
        StatsRaw,
        StatsSample,
        LogChannel,
        LogRecord,
        FollowLogsRequest,
        AckLogsRequest,
        LogBatch,
        FollowEventsRequest,
        ContainerEventAction,
        ContainerEvent,
        EventBatch,
        ExportLogsRequest,
        ExportLogsResponse,
        ContainerPort,
        ComposeLabels,
        WorkspaceMode,
        DemoScenario,
        SwitchWorkspaceRequest,
        WorkspaceModeSnapshot,
        DiscoverHostsRequest,
        SshConfigPath,
        SshCandidate,
        DiscoveryWarning,
        DiscoveryWarningCode,
        HostDiscovery,
        SelectSshAliasRequest,
        SshSelection,
        ResolveSshRequest,
        EffectiveSshConfig,
        SshAccessStatus,
        SshAccessReport,
        ConnectionStage,
        ConnectionDiagnosticCode,
        ConnectionToken,
        ConnectionRequest,
        StageDuration,
        ConnectionDiagnostic,
        ConnectionSnapshot,
        HostDraft,
        InventoryModeRequest,
        SaveHostRequest,
        RemoveHostRequest,
        InventoryConnectRequest,
        InventoryDisconnectRequest,
        HostInventory,
        SshTransportMode,
        DockerOptions,
        BeginSshRequest,
        DockerProbeStatus,
        DockerEndpointKind,
        ComposeAvailability,
        DockerProbeReport
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
        ports: vec![],
        compose: None,
        cli: None,
    };
    let detail = crate::docker::inspect::parse(&scope(), &summary.id, false, serde_json::to_vec(&json!([{
        "Id": summary.id, "Name": "/example", "Config": {"Image": "example:fixture", "Env": ["EXAMPLE_TOKEN=synthetic-value"]}, "State": {"Status": "running"}
    }])).unwrap().as_slice()).unwrap();
    let mut stats = crate::docker::stats::parse(&ContainerStatsRequest { scope: scope(), container_id: ContainerId("a".repeat(64)) }, &serde_json::to_vec(&json!({"ID":"a".repeat(64), "CPUPerc":"234.5%", "MemUsage":"2MiB / 64MiB", "MemPerc":"3.125%", "NetIO":"1.2kB / 3MB", "BlockIO":"4KiB / 5B", "PIDs":"7"})).unwrap()).unwrap();
    stats.captured_at_ms = 1_770_000_000_000.0;
    let fixtures = json!({
        "stats": stats,
        "effectiveSsh": EffectiveSshConfig { selection: SshSelection { alias: "fixture-host".into(), config_path: "/fixture/.ssh/config".into(), use_default_config: false }, executable_path: "/usr/bin/ssh".into(), hostname: "192.0.2.10".into(), user: "fixture-user".into(), port: 2222, proxy_jump: Some("fixture-jump".into()), has_proxy_command: false },
        "discovery": HostDiscovery { config_path: "/fixture/.ssh/config".into(), candidates: vec![SshCandidate { alias: "fixture-host".into(), source: "/fixture/.ssh/config".into(), line: 2 }], warnings: vec![DiscoveryWarning { code: DiscoveryWarningCode::PatternsSkipped, source: "/fixture/.ssh/config".into(), line: Some(3) }] },
        "liveWorkspace": WorkspaceModeSnapshot { mode: WorkspaceMode::Live, scenario: None, scope: None, host: None },
        "confirmationRequest": PrepareConfirmationRequest { scope: scope(), operation: ConfirmationOperation::Mutation(MutationSpec { operation: MutationOperation::Stop, container_ids: vec![ContainerId("a".repeat(64))], timeout_seconds: 10 }) },
        "policyError": AppError::new(ErrorCode::PermissionDenied).in_scope(&scope()),
        "mutationRequest": MutationRequest { scope: scope(), intent_id: IntentId(format!("i_{}", "f".repeat(32))), spec: MutationSpec { operation: MutationOperation::Stop, container_ids: vec![ContainerId("a".repeat(64))], timeout_seconds: 10 } },
        "diagnostics": DependencyDiagnostics { app_version: "0.1.0".into(), platform: "linux".into(), architecture: "x86_64".into(), ssh: SshDiagnostic { path: "/usr/bin/ssh".into(), status: SshStatus::Ready, version: Some("OpenSSH_fixture".into()), message: "OpenSSH is available.".into() }, agent: AgentDiagnostic { status: AgentStatus::Unset, message: "SSH_AUTH_SOCK is not set. Existing configured keys may still work.".into() } },
        "preferences": PreferencesSnapshot { preferences: Preferences::default(), notice: None, writable: true },
        "success": ListContainersResponse { scope: scope(), containers: vec![summary] },
        "logs": LogSnapshot {scope:scope(),container_id:ContainerId("a".repeat(64)),records:vec![LogRecord {text:"synthetic-fixture-output".into(),timestamp:Some("2026-01-01T00:00:00.000000001Z".into()),channel:LogChannel::StderrAmbiguous,truncated:false,invalid_utf8:false}],truncated:false,dropped_records:0,stderr_ambiguous:true},
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

#[test]
fn generated_demo_snapshot_is_current() {
    let host = crate::transport::demo_host();
    let mut demo_scope = scope();
    demo_scope.selection.host_id = host.id.clone();
    demo_scope.daemon_id = "demo-fixture-daemon".into();
    let response = crate::transport::fixtures::parse(
        &demo_scope,
        include_bytes!("../fixtures/demo/containers.jsonl"),
    )
    .unwrap();
    let workspace = WorkspaceModeSnapshot {
        mode: WorkspaceMode::Demo,
        scenario: Some(DemoScenario::Standard),
        scope: Some(demo_scope),
        host: Some(host),
    };
    let content =
        serde_json::to_string_pretty(&json!({"workspace":workspace,"inventory":response})).unwrap()
            + "\n";
    check_or_update(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/demo/workspace.generated.json"),
        &content,
    );
}
