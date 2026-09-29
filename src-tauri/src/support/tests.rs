use super::*;
#[test]
fn frozen_preview_expiry_and_replacement_cannot_export_another_report() {
    let mut store = PreviewStore::default();
    let old = store.prepare("first".into()).unwrap();
    let new = store.prepare("second".into()).unwrap();
    assert!(store.get(&old.id).is_err());
    assert_eq!(store.get(&new.id).unwrap(), "second");
    store.0.as_mut().unwrap().1 = Instant::now() - PREVIEW_LIFETIME;
    assert_eq!(
        store.get(&new.id).unwrap_err().code,
        ErrorCode::IntentExpired
    );
    assert!(store.0.is_none());
    assert!(store.prepare("x".repeat(MAX_BYTES + 1)).is_err());
}

#[test]
fn every_staged_error_excludes_seeded_identity_config_and_payload_secrets() {
    let secret = "SYNTHETIC_043_SECRET";
    let selection = SshSelection {
        alias: secret.into(),
        config_path: format!("/private/{secret}"),
        use_default_config: false,
    };
    let mut connection = ConnectionSnapshot {
        host_id: Some(HostId(secret.into())),
        effective: Some(EffectiveSshConfig {
            selection: selection.clone(),
            executable_path: secret.into(),
            hostname: secret.into(),
            user: secret.into(),
            port: 22,
            proxy_jump: Some(secret.into()),
            has_proxy_command: true,
        }),
        token: ConnectionToken {
            session_id: SessionId(secret.into()),
            session_generation: 1,
        },
        selection,
        state: ConnectionState::Error,
        durations: vec![
            StageDuration {
                stage: ConnectionStage::Resolve,
                duration_ms: u32::MAX
            };
            9
        ],
        diagnostic: None,
        has_jump: true,
        transport_mode: SshTransportMode::DirectFallback,
        docker_options: DockerOptions {
            executable: Some(secret.into()),
            context: Some(secret.into()),
            sudo: true,
        },
        docker: None,
    };
    let preferences = Preferences {
        trusted_config_path: Some(secret.into()),
        ssh_executable_override: Some(secret.into()),
        ..Default::default()
    };
    for code in [
        ConnectionDiagnosticCode::ResolutionFailed,
        ConnectionDiagnosticCode::UnknownHostKey,
        ConnectionDiagnosticCode::ChangedHostKey,
        ConnectionDiagnosticCode::HostKeyRejected,
        ConnectionDiagnosticCode::AuthenticationFailed,
        ConnectionDiagnosticCode::ConnectionFailed,
        ConnectionDiagnosticCode::TimedOut,
        ConnectionDiagnosticCode::OutputLimit,
        ConnectionDiagnosticCode::ProbeUnavailable,
        ConnectionDiagnosticCode::DockerUnavailable,
        ConnectionDiagnosticCode::RemoteCommandFailed,
        ConnectionDiagnosticCode::ConnectionLost,
    ] {
        connection.diagnostic = Some(ConnectionDiagnostic {
            stage: ConnectionStage::Authenticate,
            code,
        });
        let text = report(Some(&preferences), Some(&connection), &[], &[]).unwrap();
        assert!(!text.contains(secret));
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["connection"]["host"], "host-1");
        assert_eq!(value["connection"]["timings"].as_array().unwrap().len(), 3);
        assert_eq!(
            value["connection"]["timings"][0]["durationMsBucket"],
            30_000
        );
        assert_eq!(
            value["connection"]["error"]["code"],
            serde_json::to_value(code).unwrap()
        );
    }
}

#[test]
fn all_operation_errors_export_codes_without_error_messages_or_target_identities() {
    let codes = [
        ErrorCode::InvalidId,
        ErrorCode::InvalidGeneration,
        ErrorCode::HostNotFound,
        ErrorCode::ContainerNotFound,
        ErrorCode::ContainerNotStopped,
        ErrorCode::ContainerNotRunning,
        ErrorCode::TerminalClosed,
        ErrorCode::TerminalShellUnavailable,
        ErrorCode::ImageNotFound,
        ErrorCode::VolumeNotFound,
        ErrorCode::NetworkNotFound,
        ErrorCode::ComposeConfigurationUnavailable,
        ErrorCode::ComposeProjectMismatch,
        ErrorCode::ComposeVerificationExpired,
        ErrorCode::LogDriverUnsupported,
        ErrorCode::ExportFailed,
        ErrorCode::SessionNotFound,
        ErrorCode::StaleSession,
        ErrorCode::SubscriptionNotFound,
        ErrorCode::FeatureUnavailable,
        ErrorCode::PermissionDenied,
        ErrorCode::ResourceLimit,
        ErrorCode::TransportUnavailable,
        ErrorCode::InvalidResponse,
        ErrorCode::Internal,
        ErrorCode::StorageUnavailable,
        ErrorCode::StorageConflict,
        ErrorCode::InvalidPreferences,
        ErrorCode::InvalidLimits,
        ErrorCode::InvalidIntent,
        ErrorCode::IntentExpired,
        ErrorCode::Disconnected,
        ErrorCode::OperationTimedOut,
        ErrorCode::OperationCancelled,
        ErrorCode::InvalidAlias,
        ErrorCode::InvalidConfigPath,
        ErrorCode::InvalidRemoteArgument,
        ErrorCode::SshUnavailable,
        ErrorCode::SshResolutionFailed,
    ];
    for code in codes {
        let original = AppError {
            code: code.clone(),
            message: "SYNTHETIC_043_SECRET_ERROR".into(),
            scope: Some(crate::contract_tests::scope()),
        };
        let record = crate::activity::ActivityRecord {
            id: IntentId("SYNTHETIC_043_SECRET_INTENT".into()),
            host_id: HostId("SYNTHETIC_043_SECRET_HOST".into()),
            action: MutationOperation::Stop,
            targets: vec![ContainerId("SYNTHETIC_043_SECRET_CONTAINER".into())],
            started_at_ms: 0,
            updated_at_ms: u64::MAX,
            outcome: crate::activity::ActivityOutcome::Failed,
            results: vec![MutationTargetResult {
                container_id: ContainerId("SYNTHETIC_043_SECRET_CONTAINER".into()),
                outcome: MutationTargetOutcome::Failed,
                dispatched: true,
                error: Some(original.code),
            }],
        };
        let text = report(Some(&Preferences::default()), None, &[record], &[]).unwrap();
        assert!(!text.contains("SYNTHETIC_043_SECRET"));
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            json["activity"][0]["errors"][0],
            serde_json::to_value(code).unwrap()
        );
        assert_eq!(json["activity"][0]["durationMsBucket"], 3_600_000);
    }
}

#[test]
fn inaccessible_sources_still_produce_a_safe_report_without_guessing_configuration() {
    let report = report(
        None,
        None,
        &[],
        &[SourceError {
            source: Source::Preferences,
            code: ErrorCode::StorageUnavailable,
        }],
    )
    .unwrap();
    let value: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(value["transport"]["executable"], "unavailable");
    assert_eq!(value["sourceErrors"][0]["code"], "storage_unavailable");
}
