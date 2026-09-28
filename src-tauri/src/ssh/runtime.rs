//! Private short runtime paths. A flock lease distinguishes live owners; recovery never kills PIDs.
use crate::domain::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime},
};
const NAMES: [&str; 4] = ["lease", "policy.conf", "user.conf", "cm"];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    format: String,
    instance: String,
    socket: Option<(u64, u64)>,
}
fn error() -> AppError {
    AppError::new(ErrorCode::ResourceLimit)
}
fn uid() -> u32 {
    unsafe { libc::geteuid() }
}
fn token() -> Result<String, AppError> {
    let mut random = [0; 16];
    getrandom::fill(&mut random).map_err(|_| error())?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}
fn valid_token(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|m| m.is_dir() && m.uid() == uid() && m.mode() & 0o777 == 0o700)
}
fn root() -> Result<PathBuf, AppError> {
    let path = Path::new("/tmp").join(format!("containerdesk-{}", uid()));
    match DirBuilder::new().mode(0o700).create(&path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(_) => return Err(error()),
    }
    if !directory(&path) {
        return Err(error());
    }
    Ok(path)
}
fn identity(path: &Path) -> Option<(u64, u64)> {
    fs::symlink_metadata(path).ok().map(|m| (m.dev(), m.ino()))
}
fn allowed_entries(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|entries| {
        let entries: Vec<_> = entries.take(5).collect();
        entries.len() <= 4
            && entries
                .into_iter()
                .all(|entry| entry.is_ok_and(|e| NAMES.iter().any(|name| e.file_name() == *name)))
    })
}
fn remove_empty_runtime(path: &Path) {
    if !allowed_entries(path) || fs::symlink_metadata(path.join("cm")).is_ok() {
        return;
    }
    for name in ["policy.conf", "user.conf", "lease"] {
        let _ = fs::remove_file(path.join(name));
    }
    let _ = fs::remove_dir(path);
}
pub(crate) struct RuntimeLease {
    path: PathBuf,
    identity: (u64, u64),
    lease: Mutex<(File, Marker)>,
}
impl RuntimeLease {
    pub fn create() -> Result<Self, AppError> {
        static INSTANCE: OnceLock<Result<String, AppError>> = OnceLock::new();
        let instance = INSTANCE.get_or_init(token).clone()?;
        let root = root()?;
        if fs::read_dir(&root).map_err(|_| error())?.take(129).count() >= 128 {
            return Err(error());
        }
        let path = root.join(token()?);
        // Leave space for OpenSSH's temporary socket suffix on macOS's smaller sockaddr_un.
        if path.join("cm").as_os_str().len() > 80 {
            return Err(error());
        }
        DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|_| error())?;
        let Some(created) = identity(&path) else {
            let _ = fs::remove_dir(&path);
            return Err(error());
        };
        let mut file = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path.join("lease"))
        {
            Ok(file) => file,
            Err(_) => {
                let _ = fs::remove_dir(&path);
                return Err(error());
            }
        };
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let _ = fs::remove_file(path.join("lease"));
            let _ = fs::remove_dir(&path);
            return Err(error());
        }
        let marker = Marker {
            format: "containerdesk-ssh-v1".into(),
            instance,
            socket: None,
        };
        if serde_json::to_writer(&mut file, &marker).is_err() {
            let _ = fs::remove_file(path.join("lease"));
            let _ = fs::remove_dir(&path);
            unsafe {
                libc::flock(file.as_raw_fd(), libc::LOCK_UN);
            }
            return Err(error());
        }
        Ok(Self {
            path,
            identity: created,
            lease: Mutex::new((file, marker)),
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn socket_path(&self) -> PathBuf {
        self.path.join("cm")
    }
    pub fn owns_directory(&self) -> bool {
        directory(&self.path) && identity(&self.path) == Some(self.identity)
    }
    /// Only called after starting the app master in a previously absent private socket location.
    pub fn record_socket(&self) -> Result<(), AppError> {
        if !self.owns_directory() {
            return Err(error());
        }
        let metadata = fs::symlink_metadata(self.socket_path()).map_err(|_| error())?;
        if !metadata.file_type().is_socket() || metadata.uid() != uid() {
            return Err(error());
        }
        let mut lease = self.lease.lock().map_err(|_| error())?;
        let found = (metadata.dev(), metadata.ino());
        if lease.1.socket.is_some_and(|owned| owned != found) {
            return Err(error());
        }
        lease.1.socket = Some(found);
        let encoded = serde_json::to_vec(&lease.1).map_err(|_| error())?;
        lease.0.seek(SeekFrom::Start(0)).map_err(|_| error())?;
        lease.0.set_len(0).map_err(|_| error())?;
        lease.0.write_all(&encoded).map_err(|_| error())?;
        Ok(())
    }
    pub fn owns_socket(&self) -> bool {
        self.owns_directory()
            && self.lease.lock().is_ok_and(|lease| {
                fs::symlink_metadata(self.socket_path()).is_ok_and(|metadata| {
                    metadata.file_type().is_socket()
                        && metadata.uid() == uid()
                        && Some((metadata.dev(), metadata.ino())) == lease.1.socket
                })
            })
    }
    pub fn remove_dead_socket(&self) {
        if self.owns_socket() {
            let _ = fs::remove_file(self.socket_path());
        }
    }
}
impl Drop for RuntimeLease {
    fn drop(&mut self) {
        if self.owns_directory() {
            remove_empty_runtime(&self.path);
        }
        // Explicit unlock: exec inherits no lease; fork-only descendants must not prolong ownership.
        if let Ok(lease) = self.lease.lock() {
            unsafe {
                libc::flock(lease.0.as_raw_fd(), libc::LOCK_UN);
            }
        }
    }
}
/// Bounded best-effort cleanup. Preserve live leases, live sockets, unknown entries and changed identities.
pub(crate) async fn recover() {
    let Ok(root) = root() else { return };
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.take(128).flatten() {
        if entry.file_name().to_str().is_some_and(valid_token) {
            recover_directory(&entry.path()).await;
        }
    }
}
struct RecoveryLock(File);
impl Drop for RecoveryLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}
async fn recover_directory(path: &Path) {
    if !directory(path) || !allowed_entries(path) {
        return;
    }
    let Some(original) = identity(path) else {
        return;
    };
    let Ok(lease) = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path.join("lease"))
    else {
        return;
    };
    let Ok(metadata) = lease.metadata() else {
        return;
    };
    if !metadata.is_file()
        || metadata.uid() != uid()
        || metadata.mode() & 0o777 != 0o600
        || metadata.nlink() != 1
        || metadata.len() > 1024
    {
        return;
    }
    if unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return;
    }
    let mut locked = RecoveryLock(lease);
    let mut bytes = Vec::new();
    if std::io::Read::by_ref(&mut locked.0)
        .take(1025)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return;
    }
    let Ok(marker) = serde_json::from_slice::<Marker>(&bytes) else {
        return;
    };
    if marker.format != "containerdesk-ssh-v1" || !valid_token(&marker.instance) {
        return;
    }
    // Give a crashed/in-flight starter time to settle before examining its paths.
    if !metadata
        .modified()
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age >= Duration::from_secs(120))
    {
        return;
    }
    let socket = path.join("cm");
    if let Ok(metadata) = fs::symlink_metadata(&socket) {
        if !metadata.file_type().is_socket()
            || metadata.uid() != uid()
            || Some((metadata.dev(), metadata.ino())) != marker.socket
        {
            return;
        }
        match tokio::time::timeout(
            Duration::from_millis(100),
            tokio::net::UnixStream::connect(&socket),
        )
        .await
        {
            Ok(Err(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::NotFound
                ) => {}
            _ => return, // Includes a live listener, permission ambiguity and exhausted backlog.
        }
        if identity(path) != Some(original) || identity(&socket) != marker.socket {
            return;
        }
        if fs::remove_file(socket).is_err() {
            return;
        }
    }
    if identity(path) == Some(original) {
        remove_empty_runtime(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    fn age(lease: &RuntimeLease) {
        lease
            .lease
            .lock()
            .unwrap()
            .0
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(SystemTime::now() - Duration::from_secs(180)),
            )
            .unwrap();
    }
    #[tokio::test]
    async fn recovery_preserves_live_owners_and_listeners_then_removes_only_dead_owned_socket() {
        let lease = RuntimeLease::create().unwrap();
        let path = lease.path().to_path_buf();
        fs::write(path.join("policy.conf"), b"Host *\n").unwrap();
        age(&lease);
        recover_directory(&path).await;
        assert!(path.exists(), "live flock protects owner");
        let listener = UnixListener::bind(lease.socket_path()).unwrap();
        lease.record_socket().unwrap();
        age(&lease);
        drop(lease); // Socket keeps runtime files; dropping flock simulates a departed app owner.
        recover_directory(&path).await;
        assert!(
            path.exists(),
            "live daemon is never terminated during recovery"
        );
        drop(listener);
        // Concurrent test forks can briefly retain a listener FD until exec closes CLOEXEC.
        // Recovery must preserve that live listener, then clean it once it is actually dead.
        let started = std::time::Instant::now();
        while path.exists() {
            recover_directory(&path).await;
            assert!(started.elapsed() < Duration::from_secs(2));
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            !path.exists(),
            "dead owned socket and exact known entries can be removed"
        );
    }
    #[tokio::test]
    async fn recovery_retains_unknown_files_and_replaced_socket_identity() {
        let lease = RuntimeLease::create().unwrap();
        let path = lease.path().to_path_buf();
        fs::write(path.join("unrelated"), b"retain").unwrap();
        age(&lease);
        drop(lease);
        recover_directory(&path).await;
        assert_eq!(fs::read(path.join("unrelated")).unwrap(), b"retain");
        fs::remove_file(path.join("unrelated")).unwrap();
        recover_directory(&path).await;
        assert!(!path.exists());
        let lease = RuntimeLease::create().unwrap();
        let path = lease.path().to_path_buf();
        let original = UnixListener::bind(lease.socket_path()).unwrap();
        lease.record_socket().unwrap();
        fs::remove_file(lease.socket_path()).unwrap();
        let replacement = UnixListener::bind(lease.socket_path()).unwrap();
        assert!(!lease.owns_socket());
        assert!(
            lease.record_socket().is_err(),
            "cannot adopt a replacement listener"
        );
        age(&lease);
        drop(lease);
        recover_directory(&path).await;
        assert!(path.join("cm").exists());
        drop(original);
        drop(replacement);
        fs::remove_file(path.join("cm")).unwrap();
        recover_directory(&path).await;
        assert!(!path.exists());
    }
}
