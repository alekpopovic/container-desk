use super::*;
fn request() -> ContainerStatsRequest {
    ContainerStatsRequest {
        scope: crate::contract_tests::scope(),
        container_id: ContainerId("a".repeat(64)),
    }
}
fn fixture() -> serde_json::Value {
    serde_json::json!({"ID":"a".repeat(64),"CPUPerc":"250.25%","MemUsage":"1.5MiB / 2GiB","MemPerc":"0.07%","NetIO":"1.2kB / 3MB","BlockIO":"4KiB / 5B","PIDs":"7"})
}
#[test]
fn docker_cli_decimal_binary_units_and_multicore_percent_are_preserved() {
    let sample = parse(&request(), &serde_json::to_vec(&fixture()).unwrap()).unwrap();
    assert_eq!(sample.values.cpu_percent, Some(250.25));
    assert_eq!(
        sample.values.memory_usage_bytes,
        Some(1.5 * 1024.0 * 1024.0)
    );
    assert_eq!(
        sample.values.memory_limit_bytes,
        Some(2.0 * 1024_f64.powi(3))
    );
    assert_eq!(sample.values.network_rx_bytes, Some(1200.0));
    assert_eq!(sample.values.network_tx_bytes, Some(3_000_000.0));
    assert_eq!(sample.values.block_read_bytes, Some(4096.0));
    assert_eq!(sample.values.block_write_bytes, Some(5.0));
    assert_eq!(sample.values.pids, Some(7));
    assert_eq!(sample.raw.memory.as_deref(), Some("1.5MiB / 2GiB"));
    for value in [
        "1,2MB",
        "NaNMiB",
        "1e3B",
        "-1B",
        "999999999999999999EB",
        "N/A",
        "1B / 2B",
    ] {
        assert_eq!(bytes(value), None);
    }
    assert_eq!(bytes(" 2 KiB "), Some(2048.0));
}
#[test]
fn unavailable_or_unrecognized_fields_are_gaps_not_fabricated_zeroes() {
    let sample = parse(&request(),br#"{"ID":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","CPUPerc":"--","MemUsage":"N/A","PIDs":"--"}"#).unwrap();
    assert_eq!(sample.availability, StatsAvailability::Unavailable);
    assert_eq!(sample.values, StatsValues::default());
    assert_eq!(sample.raw.cpu.as_deref(), Some("--"));
    assert_eq!(
        parse(&request(), b"").unwrap().availability,
        StatsAvailability::Unavailable
    );
    let mut fixture = fixture();
    fixture["MemUsage"] = "0B / 0B".into();
    fixture["MemPerc"] = "0.00%".into();
    let sample = parse(&request(), &serde_json::to_vec(&fixture).unwrap()).unwrap();
    assert_eq!(sample.values.memory_usage_bytes, None);
    assert_eq!(sample.values.memory_percent, None);
    assert!(sample.captured_at_ms.is_finite());
}
#[test]
fn malformed_foreign_or_oversized_stats_never_cross_ipc() {
    for (field, value) in [
        ("ID", "b".repeat(64)),
        ("CPUPerc", "\x1bsecret".into()),
        ("MemUsage", "x".repeat(129)),
    ] {
        let mut f = fixture();
        f[field] = value.into();
        assert!(parse(&request(), &serde_json::to_vec(&f).unwrap()).is_err());
    }
    let f = serde_json::to_vec(&fixture()).unwrap();
    assert!(parse(&request(), &[f.clone(), b"\n".to_vec(), f].concat()).is_err());
    assert!(parse(&request(), &vec![b' '; 16385]).is_err());
    assert!(
        !format!(
            "{:?}",
            parse(&request(), &serde_json::to_vec(&fixture()).unwrap()).unwrap()
        )
        .contains("1.5MiB")
    );
}
#[test]
fn host_slot_is_exclusive_across_containers_and_generations_and_released_on_drop() {
    let slots = Slots::default();
    let host = request().scope.selection.host_id;
    let guard = slots.acquire(&host).unwrap();
    assert!(matches!(
        slots.acquire(&host),
        Err(AppError {
            code: ErrorCode::ResourceLimit,
            ..
        })
    ));
    let second = slots
        .acquire(&HostId(format!("h_{}", "2".repeat(32))))
        .unwrap();
    let third = slots
        .acquire(&HostId(format!("h_{}", "3".repeat(32))))
        .unwrap();
    assert!(
        slots
            .acquire(&HostId(format!("h_{}", "4".repeat(32))))
            .is_err()
    );
    drop((guard, second, third));
    assert!(slots.acquire(&host).is_ok());
    let plan = registry::read(&ReadOperation::ContainerStats {
        container_id: request().container_id,
    })
    .unwrap();
    assert_eq!(
        plan.args()[1..7],
        [
            "stats",
            "--no-stream",
            "--no-trunc",
            "--format",
            "{{json .}}",
            "--"
        ]
    );
    assert!(
        registry::read(&ReadOperation::ContainerStats {
            container_id: ContainerId("--all".into())
        })
        .is_err()
    );
}
