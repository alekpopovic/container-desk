//! Versioned, bounded, non-secret settings with an injectable persistence adapter.
mod filesystem;
use crate::domain::*;
pub use filesystem::FileStorage;
use serde::Deserialize;
use std::{collections::HashSet, io, path::Path};

pub const MAX_BYTES: usize = 1024 * 1024;

pub trait Storage: Send {
    fn read(&self, previous: bool) -> io::Result<Option<Vec<u8>>>;
    fn preserve_corrupt(&self) -> io::Result<()>;
    /// Keep `previous` before committing `current`, atomically for each file.
    fn commit(&self, current: &[u8], previous: Option<&[u8]>) -> io::Result<()>;
}

pub struct SettingsStore {
    adapter: Box<dyn Storage>,
    snapshot: PreferencesSnapshot,
    /// Only known-good bytes can become the recoverable previous version.
    valid_bytes: Option<Vec<u8>>,
}
fn unavailable(_: io::Error) -> AppError {
    AppError::new(ErrorCode::StorageUnavailable)
}
fn invalid() -> AppError {
    AppError::new(ErrorCode::InvalidPreferences)
}

pub fn validate(preferences: &Preferences) -> Result<(), AppError> {
    let safe_text = |s: &str, max: usize| s.len() <= max && !s.chars().any(char::is_control);
    if preferences.schema_version != 2 || preferences.hosts.len() > 1000 {
        return Err(invalid());
    }
    let mut ids = HashSet::new();
    let mut aliases = HashSet::new();
    for host in &preferences.hosts {
        host.id.validate().map_err(|_| invalid())?;
        if crate::ssh::validate_alias(&host.alias).is_err()
            || !ids.insert(&host.id)
            || !aliases.insert(&host.alias)
            || host.display_name.is_empty()
            || !safe_text(&host.display_name, 256)
            || !safe_text(&host.group, 128)
            || host.labels.len() > 32
            || !host.labels.iter().all(|s| safe_text(s, 128))
        {
            return Err(invalid());
        }
    }
    if preferences
        .selected_host_id
        .as_ref()
        .is_some_and(|id| !ids.contains(id))
    {
        return Err(invalid());
    }
    for reference in [
        &preferences.trusted_config_path,
        &preferences.ssh_executable_override,
    ]
    .into_iter()
    .flatten()
    {
        if !Path::new(reference).is_absolute() || !safe_text(reference, 4096) {
            return Err(invalid());
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct V1 {
    schema_version: u32,
    theme: Theme,
    hosts: Vec<SavedHost>,
    selected_host_id: Option<HostId>,
    trusted_config_path: Option<String>,
    ssh_executable_override: Option<String>,
}
enum Decoded {
    Current(Preferences),
    Migrated(Preferences),
    Unsupported,
    Corrupt,
}
fn decode(bytes: &[u8]) -> Decoded {
    if bytes.len() > MAX_BYTES {
        return Decoded::Corrupt;
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Decoded::Corrupt;
    };
    let decoded = match value.get("schemaVersion").and_then(|v| v.as_u64()) {
        Some(2) => serde_json::from_value::<Preferences>(value)
            .ok()
            .map(|v| (v, false)),
        Some(1) => serde_json::from_value::<V1>(value).ok().map(|v| {
            debug_assert_eq!(v.schema_version, 1);
            (
                Preferences {
                    schema_version: 2,
                    revision: 0,
                    theme: v.theme,
                    hosts: v.hosts,
                    selected_host_id: v.selected_host_id,
                    trusted_config_path: v.trusted_config_path,
                    ssh_executable_override: v.ssh_executable_override,
                },
                true,
            )
        }),
        Some(version) if version > 2 => return Decoded::Unsupported,
        _ => None,
    };
    match decoded {
        Some((preferences, migrated)) if validate(&preferences).is_ok() => {
            if migrated {
                Decoded::Migrated(preferences)
            } else {
                Decoded::Current(preferences)
            }
        }
        _ => Decoded::Corrupt,
    }
}

impl SettingsStore {
    pub fn load(adapter: Box<dyn Storage>) -> Result<Self, AppError> {
        let primary = adapter.read(false).map_err(unavailable)?;
        let mut store = Self {
            adapter,
            snapshot: PreferencesSnapshot {
                preferences: Preferences::default(),
                notice: None,
                writable: true,
            },
            valid_bytes: None,
        };
        if let Some(bytes) = primary {
            match decode(&bytes) {
                Decoded::Current(preferences) => {
                    store.snapshot.preferences = preferences;
                    store.valid_bytes = Some(bytes);
                }
                Decoded::Migrated(preferences) => {
                    store.snapshot.preferences = preferences;
                    store.snapshot.notice = Some(StorageNotice::Migrated);
                    store.valid_bytes = Some(bytes);
                    store.persist()?;
                }
                Decoded::Unsupported => {
                    store.snapshot.notice = Some(StorageNotice::UnsupportedSchema);
                    store.snapshot.writable = false;
                }
                Decoded::Corrupt => {
                    store.adapter.preserve_corrupt().map_err(unavailable)?;
                    let backup = store.adapter.read(true).map_err(unavailable)?;
                    match backup.as_deref().map(decode) {
                        Some(Decoded::Current(p) | Decoded::Migrated(p)) => {
                            store.snapshot.preferences = p;
                            store.valid_bytes = backup;
                            store.snapshot.notice = Some(StorageNotice::RecoveredPrevious);
                        }
                        _ => store.snapshot.notice = Some(StorageNotice::ResetAfterCorruption),
                    }
                    // Preserve the existing previous file; never rotate corrupt primary bytes over it.
                    store.persist()?;
                }
            }
        }
        Ok(store)
    }
    fn persist(&mut self) -> Result<(), AppError> {
        let bytes = serde_json::to_vec_pretty(&self.snapshot.preferences).map_err(|_| invalid())?;
        if bytes.len() > MAX_BYTES {
            return Err(invalid());
        }
        if let Err(error) = self.adapter.commit(&bytes, self.valid_bytes.as_deref()) {
            // A rename may have succeeded before fsync failed. Do not risk another overwrite.
            self.snapshot.writable = false;
            return Err(unavailable(error));
        }
        self.valid_bytes = Some(bytes);
        Ok(())
    }
    pub fn snapshot(&self) -> PreferencesSnapshot {
        self.snapshot.clone()
    }
    pub fn replace(
        &mut self,
        mut preferences: Preferences,
        expected_revision: u32,
    ) -> Result<PreferencesSnapshot, AppError> {
        if !self.snapshot.writable {
            return Err(AppError::new(ErrorCode::StorageUnavailable));
        }
        if expected_revision != self.snapshot.preferences.revision {
            return Err(AppError::new(ErrorCode::StorageConflict));
        }
        validate(&preferences)?;
        preferences.revision = expected_revision.checked_add(1).ok_or_else(invalid)?;
        let previous = self.snapshot.preferences.clone();
        self.snapshot.preferences = preferences;
        if let Err(error) = self.persist() {
            self.snapshot.preferences = previous;
            return Err(error);
        }
        Ok(self.snapshot())
    }
    pub fn set_theme(&mut self, request: SetThemeRequest) -> Result<PreferencesSnapshot, AppError> {
        let mut preferences = self.snapshot.preferences.clone();
        preferences.theme = request.theme;
        self.replace(preferences, request.expected_revision)
    }
}

#[cfg(test)]
pub(crate) mod tests;
