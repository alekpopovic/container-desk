//! Startup-only, local numeric limits. Configuration can reduce established hard ceilings.
use crate::domain::*;
use std::{fs::OpenOptions, io::Read, os::unix::fs::OpenOptionsExt, path::Path, sync::OnceLock};
static CURRENT: OnceLock<ResourceLimitsReport> = OnceLock::new();
impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            log_lines: crate::docker::logs::MAX_RECORDS as u32,
            log_bytes: 8 * 1024 * 1024,
            stats_history: 360,
            active_hosts: 1,
            concurrent_jobs: 4,
        }
    }
}
impl ResourceLimits {
    pub fn valid(&self) -> bool {
        (1000..=20_000).contains(&self.log_lines)
            && (256 * 1024..=8 * 1024 * 1024).contains(&self.log_bytes)
            && (60..=360).contains(&self.stats_history)
            && self.active_hosts <= 1
            && (2..=4).contains(&self.concurrent_jobs)
    }
}
fn load(directory: &Path) -> ResourceLimitsReport {
    let read = || -> Result<ResourceLimits, ()> {
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(directory.join("resource-limits.json"))
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ResourceLimits::default());
            }
            Err(_) => return Err(()),
        };
        let metadata = file.metadata().map_err(|_| ())?;
        if !metadata.is_file() || metadata.len() > 4096 {
            return Err(());
        }
        let mut bytes = Vec::new();
        file.take(4097).read_to_end(&mut bytes).map_err(|_| ())?;
        if bytes.len() > 4096 {
            return Err(());
        }
        let limits: ResourceLimits = serde_json::from_slice(&bytes).map_err(|_| ())?;
        if !limits.valid() {
            return Err(());
        }
        Ok(limits)
    };
    match read() {
        Ok(limits) => ResourceLimitsReport {
            limits,
            configuration_ignored: false,
        },
        Err(()) => ResourceLimitsReport {
            limits: ResourceLimits::default(),
            configuration_ignored: true,
        },
    }
}
pub fn initialize(directory: &Path) {
    let _ = CURRENT.set(load(directory));
}
pub fn report() -> &'static ResourceLimitsReport {
    CURRENT.get_or_init(|| ResourceLimitsReport {
        limits: ResourceLimits::default(),
        configuration_ignored: false,
    })
}
pub fn current() -> &'static ResourceLimits {
    &report().limits
}
pub fn admit_host() -> Result<(), AppError> {
    if current().active_hosts == 0 {
        Err(AppError::new(ErrorCode::ResourceLimit))
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_limits_reject_unbounded_unknown_and_special_files() {
        let directory = std::env::temp_dir().join(format!(
            "containerdesk-045-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("resource-limits.json");
        assert!(!load(&directory).configuration_ignored);
        std::fs::write(&path, br#"{"logLines":1000,"logBytes":262144,"statsHistory":60,"activeHosts":0,"concurrentJobs":2}"#).unwrap();
        let result = load(&directory);
        assert!(!result.configuration_ignored);
        assert_eq!(result.limits.log_lines, 1000);
        assert_eq!(result.limits.active_hosts, 0);
        for invalid in [
            r#"{"logLines":20001}"#,
            r#"{"logBytes":999999999}"#,
            r#"{"statsHistory":0}"#,
            r#"{"activeHosts":2}"#,
            r#"{"concurrentJobs":0}"#,
            r#"{"concurrentJobs":5}"#,
            r#"{"unknown":1}"#,
        ] {
            std::fs::write(&path, invalid).unwrap();
            let result = load(&directory);
            assert!(result.configuration_ignored);
            assert_eq!(result.limits.concurrent_jobs, 4);
        }
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("/dev/zero", &path).unwrap();
        assert!(load(&directory).configuration_ignored);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
