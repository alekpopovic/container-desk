use super::*;
use std::{
    collections::HashMap,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Files {
    values: HashMap<bool, Vec<u8>>,
    archived: Vec<Vec<u8>>,
    fail: bool,
}
#[derive(Clone, Default)]
pub(crate) struct MemoryStorage(Arc<Mutex<Files>>);
impl Storage for MemoryStorage {
    fn read(&self, previous: bool) -> io::Result<Option<Vec<u8>>> {
        Ok(self.0.lock().unwrap().values.get(&previous).cloned())
    }
    fn preserve_corrupt(&self) -> io::Result<()> {
        let mut files = self.0.lock().unwrap();
        let original = files.values[&false].clone();
        files.archived.push(original);
        Ok(())
    }
    fn commit(&self, current: &[u8], previous: Option<&[u8]>) -> io::Result<()> {
        let mut files = self.0.lock().unwrap();
        if let Some(previous) = previous {
            files.values.insert(true, previous.to_vec());
        }
        if files.fail {
            return Err(io::Error::other(
                "injected interruption before primary replacement",
            ));
        }
        files.values.insert(false, current.to_vec());
        Ok(())
    }
}
pub(crate) fn memory_store() -> SettingsStore {
    SettingsStore::load(Box::<MemoryStorage>::default()).unwrap()
}
fn preferences() -> Preferences {
    Preferences {
        hosts: vec![
            SavedHost {
                id: HostId(format!("h_{}", "1".repeat(32))),
                alias: "lab-one".into(),
                display_name: "First".into(),
                group: "Lab".into(),
                favorite: true,
                ssh: None,
                docker: Default::default(),
                labels: vec!["test".into()],
                read_only: true,
            },
            SavedHost {
                id: HostId(format!("h_{}", "2".repeat(32))),
                alias: "lab-two".into(),
                display_name: "Second".into(),
                group: "Other".into(),
                favorite: false,
                ssh: None,
                docker: Default::default(),
                labels: vec![],
                read_only: false,
            },
        ],
        selected_host_id: Some(HostId(format!("h_{}", "2".repeat(32)))),
        trusted_config_path: Some("/tmp/reference-only/config".into()),
        ssh_executable_override: Some("/usr/bin/ssh".into()),
        ..Preferences::default()
    }
}
struct TestDir(std::path::PathBuf);
impl TestDir {
    fn new() -> Self {
        let name = format!(
            "containerdesk-storage-{}-{}",
            std::process::id(),
            crate::test_directory_suffix()
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn native_files_round_trip_two_hosts_with_private_permissions_and_previous_version() {
    let dir = TestDir::new();
    let mut store = SettingsStore::load(Box::new(FileStorage::open(&dir.0).unwrap())).unwrap();
    store.replace(preferences(), 0).unwrap();
    store
        .set_theme(SetThemeRequest {
            expected_revision: 1,
            theme: Theme::Dark,
        })
        .unwrap();
    assert!(
        FileStorage::open(&dir.0).is_err(),
        "a second process/store must not overwrite an active store"
    );
    drop(store);
    let reopened = SettingsStore::load(Box::new(FileStorage::open(&dir.0).unwrap())).unwrap();
    let p = reopened.snapshot().preferences;
    assert_eq!(p.theme, Theme::Dark);
    assert_eq!(p.hosts, preferences().hosts);
    assert_ne!(p.hosts[0].read_only, p.hosts[1].read_only);
    let bytes = fs::read(dir.0.join("preferences/settings.json")).unwrap();
    let previous: Preferences = serde_json::from_slice(
        &fs::read(dir.0.join("preferences/settings.previous.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(previous.theme, Theme::System);
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let keys = json
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            "hosts",
            "revision",
            "schemaVersion",
            "selectedHostId",
            "sshExecutableOverride",
            "theme",
            "trustedConfigPath"
        ]
    );
    for path in ["settings.json", "settings.previous.json", "settings.lock"] {
        assert_eq!(
            fs::metadata(dir.0.join("preferences").join(path))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    assert_eq!(
        fs::metadata(dir.0.join("preferences"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

#[test]
fn interrupted_commit_preserves_original_and_memory_state() {
    let adapter = MemoryStorage::default();
    let mut store = SettingsStore::load(Box::new(adapter.clone())).unwrap();
    store.replace(preferences(), 0).unwrap();
    let original = adapter.read(false).unwrap().unwrap();
    adapter.0.lock().unwrap().fail = true;
    assert_eq!(
        store
            .set_theme(SetThemeRequest {
                expected_revision: 1,
                theme: Theme::Dark
            })
            .unwrap_err()
            .code,
        ErrorCode::StorageUnavailable
    );
    assert_eq!(adapter.read(false).unwrap().unwrap(), original);
    assert_eq!(store.snapshot().preferences.theme, Theme::System);
    assert!(!store.snapshot().writable);
    adapter.0.lock().unwrap().fail = false;
    let reopened = SettingsStore::load(Box::new(adapter)).unwrap();
    assert_eq!(reopened.snapshot().preferences.hosts, preferences().hosts);
}

#[test]
fn corrupt_json_recovers_previous_and_preserves_exact_original_on_disk() {
    let dir = TestDir::new();
    let mut store = SettingsStore::load(Box::new(FileStorage::open(&dir.0).unwrap())).unwrap();
    store.replace(preferences(), 0).unwrap();
    store
        .set_theme(SetThemeRequest {
            expected_revision: 1,
            theme: Theme::Dark,
        })
        .unwrap();
    drop(store);
    let damaged = b"{interrupted invalid json";
    fs::write(dir.0.join("preferences/settings.json"), damaged).unwrap();
    let restored = SettingsStore::load(Box::new(FileStorage::open(&dir.0).unwrap())).unwrap();
    assert_eq!(
        restored.snapshot().notice,
        Some(StorageNotice::RecoveredPrevious)
    );
    assert_eq!(restored.snapshot().preferences.hosts, preferences().hosts);
    assert_eq!(
        fs::read(dir.0.join("preferences/settings.corrupt-0.json")).unwrap(),
        damaged
    );
}

#[test]
fn migration_retains_original_v1_and_unknown_future_schema_cannot_be_overwritten() {
    let adapter = MemoryStorage::default();
    let mut v1 = serde_json::to_value(preferences()).unwrap();
    v1["schemaVersion"] = 1.into();
    v1.as_object_mut().unwrap().remove("revision");
    let original = serde_json::to_vec(&v1).unwrap();
    adapter
        .0
        .lock()
        .unwrap()
        .values
        .insert(false, original.clone());
    let store = SettingsStore::load(Box::new(adapter.clone())).unwrap();
    assert_eq!(store.snapshot().notice, Some(StorageNotice::Migrated));
    assert_eq!(store.snapshot().preferences.hosts, preferences().hosts);
    assert_eq!(adapter.read(true).unwrap().unwrap(), original);
    let future = b"{\"schemaVersion\":999}".to_vec();
    adapter
        .0
        .lock()
        .unwrap()
        .values
        .insert(false, future.clone());
    let mut store = SettingsStore::load(Box::new(adapter.clone())).unwrap();
    assert_eq!(
        store.snapshot().notice,
        Some(StorageNotice::UnsupportedSchema)
    );
    assert!(
        store
            .set_theme(SetThemeRequest {
                expected_revision: 0,
                theme: Theme::Dark
            })
            .is_err()
    );
    assert_eq!(adapter.read(false).unwrap().unwrap(), future);
}

#[test]
fn no_backup_yields_visible_reset_and_stale_writes_are_rejected() {
    let adapter = MemoryStorage::default();
    adapter
        .0
        .lock()
        .unwrap()
        .values
        .insert(false, b"broken".to_vec());
    let mut store = SettingsStore::load(Box::new(adapter.clone())).unwrap();
    assert_eq!(
        store.snapshot().notice,
        Some(StorageNotice::ResetAfterCorruption)
    );
    assert_eq!(adapter.0.lock().unwrap().archived[0], b"broken");
    store.replace(preferences(), 0).unwrap();
    assert_eq!(
        store.replace(preferences(), 0).unwrap_err().code,
        ErrorCode::StorageConflict
    );
}

#[test]
fn secret_fields_invalid_references_aliases_duplicates_and_oversized_data_are_rejected() {
    let mut value = serde_json::to_value(preferences()).unwrap();
    value["privateKey"] = "SECRET_SENTINEL".into();
    assert!(serde_json::from_value::<Preferences>(value).is_err());
    let mut p = preferences();
    p.hosts[1].id = p.hosts[0].id.clone();
    assert!(validate(&p).is_err());
    p = preferences();
    p.hosts[0].alias = "-oProxyCommand=anything".into();
    assert!(validate(&p).is_err());
    p = preferences();
    p.trusted_config_path = Some("relative/config".into());
    assert!(validate(&p).is_err());
    p = preferences();
    p.hosts[0].display_name = "x".repeat(257);
    assert!(validate(&p).is_err());
    assert!(matches!(
        decode(&vec![b' '; MAX_BYTES + 1]),
        Decoded::Corrupt
    ));
}

#[test]
fn symlinks_and_orphan_partial_writes_do_not_replace_valid_settings() {
    let dir = TestDir::new();
    let mut store = SettingsStore::load(Box::new(FileStorage::open(&dir.0).unwrap())).unwrap();
    store.replace(preferences(), 0).unwrap();
    drop(store);
    fs::write(dir.0.join("preferences/.write-orphan"), b"partial").unwrap();
    let restored = SettingsStore::load(Box::new(FileStorage::open(&dir.0).unwrap())).unwrap();
    assert_eq!(restored.snapshot().preferences.hosts, preferences().hosts);
    drop(restored);
    fs::rename(
        dir.0.join("preferences/settings.json"),
        dir.0.join("original.json"),
    )
    .unwrap();
    symlink(
        dir.0.join("original.json"),
        dir.0.join("preferences/settings.json"),
    )
    .unwrap();
    assert!(SettingsStore::load(Box::new(FileStorage::open(&dir.0).unwrap())).is_err());
    assert!(dir.0.join("original.json").exists());
}

#[test]
fn schema_two_hosts_migrate_without_changing_ids_or_original_bytes() {
    let adapter = MemoryStorage::default();
    let mut old = serde_json::to_value(preferences()).unwrap();
    old["schemaVersion"] = 2.into();
    for host in old["hosts"].as_array_mut().unwrap() {
        for field in ["favorite", "ssh", "docker"] {
            host.as_object_mut().unwrap().remove(field);
        }
    }
    let original = serde_json::to_vec(&old).unwrap();
    adapter
        .0
        .lock()
        .unwrap()
        .values
        .insert(false, original.clone());
    let store = SettingsStore::load(Box::new(adapter.clone())).unwrap();
    assert_eq!(store.snapshot().notice, Some(StorageNotice::Migrated));
    let saved = store.snapshot().preferences;
    assert_eq!(saved.schema_version, 3);
    assert_eq!(saved.hosts[0].id, preferences().hosts[0].id);
    assert!(!saved.hosts[0].favorite);
    assert!(saved.hosts[0].ssh.is_none());
    assert_eq!(adapter.read(true).unwrap().unwrap(), original);
}

#[test]
fn unicode_paths_and_denied_writes_keep_existing_settings() {
    let dir = TestDir::new();
    let path = dir.0.join("Korisnik Željko/Podaci aplikacije 日本語");
    let mut store = SettingsStore::load(Box::new(FileStorage::open(&path).unwrap())).unwrap();
    store.replace(preferences(), 0).unwrap();
    let before = fs::read(path.join("preferences/settings.json")).unwrap();
    // A real Unix permission denial requires an unprivileged test process.
    if unsafe { libc::geteuid() } != 0 {
        fs::set_permissions(path.join("preferences"), fs::Permissions::from_mode(0o500)).unwrap();
        let failed = store.set_theme(SetThemeRequest {
            expected_revision: 1,
            theme: Theme::Dark,
        });
        fs::set_permissions(path.join("preferences"), fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(failed.unwrap_err().code, ErrorCode::StorageUnavailable);
        assert_eq!(
            fs::read(path.join("preferences/settings.json")).unwrap(),
            before
        );
        assert!(!store.snapshot().writable);
    }
    drop(store);
    let reopened = SettingsStore::load(Box::new(FileStorage::open(&path).unwrap())).unwrap();
    assert_eq!(reopened.snapshot().preferences.hosts, preferences().hosts);
}
