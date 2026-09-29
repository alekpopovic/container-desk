use super::*;
fn event(n: u64) -> ContainerEvent {
    ContainerEvent {
        actor_id: ContainerId("a".repeat(64)),
        action: ContainerEventAction::Start,
        timestamp_unix_nanos: n.to_string(),
    }
}
#[test]
fn events_retain_only_typed_actor_action_exact_timestamp_and_reject_bad_records() {
    let raw = serde_json::json!({"Type":"container","Action":"start","Actor":{"ID":"a".repeat(64),"Attributes":{"token":"synthetic-event-private"}},"time":1760000000u64,"timeNano":1760000000123456789u64});
    let parsed = parse(&serde_json::to_vec(&raw).unwrap()).unwrap().unwrap();
    assert_eq!(parsed.timestamp_unix_nanos, "1760000000123456789");
    assert!(
        !serde_json::to_string(&parsed)
            .unwrap()
            .contains("synthetic-event-private")
    );
    for raw in [
        serde_json::json!({"Type":"container","Action":"start","Actor":{"ID":"short"},"time":1}),
        serde_json::json!({"Type":"container","Action":"start","Actor":{"ID":"a".repeat(64)},"time":1,"timeNano":2}),
    ] {
        assert!(parse(&serde_json::to_vec(&raw).unwrap()).is_err());
    }
    assert!(parse(b"ssh banner").is_err());
    assert!(parse(&vec![b'x'; 16385]).is_err());
}
#[test]
fn duplicate_and_burst_retention_is_bounded_and_loss_remains_visible() {
    let mut queue = EventQueue::default();
    for _ in 0..5000 {
        queue.push(event(1));
    }
    assert_eq!(queue.events.len(), 1);
    assert_eq!(queue.dropped, 0);
    for i in 2..10002 {
        queue.push(event(i));
    }
    assert_eq!(queue.events.len(), 512);
    assert_eq!(queue.recent.len(), 1024);
    assert_eq!(queue.seen.len(), 1024);
    let (batch, dropped) = queue.batch();
    assert_eq!(batch.len(), 64);
    assert_eq!(dropped, 9489);
    let queue = Arc::new(Mutex::new(queue));
    let sink = line_sink(queue.clone());
    sink(Stream::Stderr, b"private diagnostic".to_vec(), false);
    sink(Stream::Stdout, vec![], true);
    assert_eq!(queue.lock().unwrap().dropped, 2);
}

#[test]
#[ignore = "reproducible synthetic pressure benchmark"]
fn pressure_event_queue_benchmark() {
    let started = std::time::Instant::now();
    let mut queue = EventQueue::default();
    let mut peak = 0;
    for index in 0..200_000 {
        queue.push(event(index));
        peak = peak.max(queue.events.len());
    }
    let (batch, dropped) = queue.batch();
    println!(
        "PRESSURE_EVENT_QUEUE {}",
        serde_json::json!({"submitted":200000,"peakEvents":peak,"dedupeEntries":queue.seen.len(),"batchEvents":batch.len(),"dropped":dropped,"elapsedMs":started.elapsed().as_secs_f64()*1000.0})
    );
    assert!(peak <= 512 && batch.len() <= 64 && dropped > 0 && queue.seen.len() <= 1024);
}
