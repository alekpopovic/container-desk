use super::*;
#[test]
fn plugin_and_inspected_labels_keep_same_service_in_distinct_unverified_projects() {
    let mut projects=plugin_projects(br#"[{"Name":"alpha","Status":"running(1)","ConfigFiles":"/remote/deleted.yml"},{"Name":"beta","Status":"created(1)","ConfigFiles":"N/A"}]"#).unwrap();
    for (i, name) in ["alpha", "beta"].iter().enumerate() {
        let id = ContainerId(format!("{:064x}", i + 1));
        let record = serde_json::json!({"id":id,"name":format!("/{name}-web-1"),"state":"created","project":name,"service":"web","configFiles":"/home/local-looking/compose.yml,/missing/override.yml","workingDir":"/remote/missing"});
        merge_labels(&serde_json::to_vec(&record).unwrap(), &[id], &mut projects).unwrap();
    }
    assert_eq!(projects.len(), 2);
    for (name, p) in projects {
        assert_eq!(p.name, name);
        assert!(p.from_plugin && p.from_labels);
        assert_eq!(p.instances[0].service.as_deref(), Some("web"));
        assert_eq!(p.configuration, ComposeConfigurationStatus::Unverified);
        assert!(
            p.config_files_reported
                .iter()
                .any(|s| s.contains("/home/local-looking"))
        );
    }
}
#[test]
fn labels_work_without_plugin_and_bulk_projection_rejects_foreign_or_duplicate_ids() {
    let id = ContainerId("a".repeat(64));
    let raw = serde_json::json!({"id":id,"name":"/single","state":"exited","project":"only-labels","service":"web","configFiles":null,"workingDir":null});
    let bytes = serde_json::to_vec(&raw).unwrap();
    let mut projects = BTreeMap::new();
    merge_labels(&bytes, std::slice::from_ref(&id), &mut projects).unwrap();
    assert!(!projects["only-labels"].from_plugin);
    assert!(projects["only-labels"].config_files_reported.is_empty());
    let duplicate = [bytes.clone(), bytes.clone()].join(&b'\n');
    assert!(merge_labels(&duplicate, std::slice::from_ref(&id), &mut BTreeMap::new()).is_err());
    assert!(merge_labels(&bytes, &[ContainerId("b".repeat(64))], &mut BTreeMap::new()).is_err());
    let plan = registry::read(&ReadOperation::InspectComposeLabels {
        container_ids: vec![id],
    })
    .unwrap();
    assert_eq!(plan.args()[5], LABEL_TEMPLATE);
    assert!(!LABEL_TEMPLATE.contains("Env"));
    assert!(!LABEL_TEMPLATE.contains("{{json .}}"));
    assert!(
        registry::read(&ReadOperation::InspectComposeLabels {
            container_ids: vec![ContainerId("--all; anything".into())]
        })
        .is_err()
    );
}
#[test]
fn reported_paths_are_opaque_and_bounds_reject_malformed_metadata() {
    assert!(plugin_projects(b"not-json").is_err());
    assert!(plugin_projects(br#"[{"Name":"p"},{"Name":"p"}]"#).is_err());
    assert!(plugin_projects(&vec![b'x'; MAX_BYTES + 1]).is_err());
    let p = plugin_projects(
        br#"[{"Name":"p","ConfigFiles":"../relative,https://untrusted.invalid,~/compose.yml"}]"#,
    )
    .unwrap();
    assert_eq!(p["p"].configuration, ComposeConfigurationStatus::Unverified);
    assert_eq!(
        p["p"].config_files_reported[0],
        "../relative,https://untrusted.invalid,~/compose.yml"
    );
    let labels = BTreeMap::from([
        ("com.docker.compose.project".into(), "p".into()),
        ("com.docker.compose.service".into(), "web".into()),
        ("private".into(), "synthetic-private".into()),
    ]);
    assert!(
        !serde_json::to_string(&group_labels(&labels))
            .unwrap()
            .contains("synthetic-private")
    );
}
