use super::{MAX_BYTES, Storage};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static SERIAL: AtomicU64 = AtomicU64::new(0);
pub struct FileStorage {
    directory: PathBuf,
    _lock: File,
}
fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "Unsafe settings file")
}
fn check_owner(file: &File, directory: bool) -> io::Result<()> {
    let metadata = file.metadata()?;
    // SAFETY: geteuid takes no pointers and cannot fail.
    if metadata.uid() != unsafe { libc::geteuid() }
        || metadata.is_dir() != directory
        || (!directory && (!metadata.is_file() || metadata.nlink() != 1))
    {
        return Err(refused());
    }
    file.set_permissions(fs::Permissions::from_mode(if directory {
        0o700
    } else {
        0o600
    }))
}
impl FileStorage {
    pub fn open(app_data: &Path) -> io::Result<Self> {
        fs::create_dir_all(app_data)?;
        let directory = app_data.join("preferences");
        match fs::DirBuilder::new().mode(0o700).create(&directory) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e),
        }
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(&directory)?;
        check_owner(&dir, true)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(directory.join("settings.lock"))?;
        check_owner(&lock, false)?;
        // SAFETY: the owned File's descriptor stays valid for the lifetime of this adapter.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            directory,
            _lock: lock,
        })
    }
    fn atomic_write(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        let temp = self.directory.join(format!(
            ".write-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temp)?;
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(&temp, self.directory.join(name))?;
            File::open(&self.directory)?.sync_all()
        })();
        // Only this exact app-created temporary path is eligible for cleanup.
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}
impl Storage for FileStorage {
    fn read(&self, previous: bool) -> io::Result<Option<Vec<u8>>> {
        let path = self.directory.join(if previous {
            "settings.previous.json"
        } else {
            "settings.json"
        });
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        check_owner(&file, false)?;
        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Settings exceed size limit",
            ));
        }
        Ok(Some(bytes))
    }
    fn preserve_corrupt(&self) -> io::Result<()> {
        let bytes = self
            .read(false)?
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        // Exclusive recovery copies retain the exact damaged bytes without touching the original.
        for n in 0..16 {
            let destination = self.directory.join(format!("settings.corrupt-{n}.json"));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&destination)
            {
                Ok(mut file) => {
                    let result = file
                        .write_all(&bytes)
                        .and_then(|()| file.sync_all())
                        .and_then(|()| File::open(&self.directory)?.sync_all());
                    if result.is_err() {
                        let _ = fs::remove_file(destination);
                    }
                    return result;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::other("Recovery archive limit reached"))
    }
    fn commit(&self, current: &[u8], previous: Option<&[u8]>) -> io::Result<()> {
        if let Some(previous) = previous {
            self.atomic_write("settings.previous.json", previous)?;
        }
        self.atomic_write("settings.json", current)
    }
}

impl Drop for FileStorage {
    fn drop(&mut self) {
        // Release ownership now, including the brief fork-before-exec window in another thread.
        // Relying only on close can leave flock held by a just-forked child's inherited descriptor.
        // SAFETY: the lock File still owns a valid descriptor while this destructor runs.
        unsafe {
            libc::flock(self._lock.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_drop_releases_lock_even_while_an_inherited_descriptor_exists() {
        let path = std::env::temp_dir().join(format!(
            "containerdesk-lock-{}-{}",
            std::process::id(),
            crate::test_directory_suffix()
        ));
        fs::create_dir(&path).unwrap();
        let store = FileStorage::open(&path).unwrap();
        let inherited = store._lock.try_clone().unwrap();
        drop(store);
        let reopened = FileStorage::open(&path).unwrap();
        drop(reopened);
        drop(inherited);
        fs::remove_dir_all(path).unwrap();
    }
}
