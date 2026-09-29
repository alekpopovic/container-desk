//! Explicit user-chosen file export. No renderer-provided path, raw diagnostics or persistent history.
use crate::domain::*;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
const MAX_BYTES: usize = 8 * 1024 * 1024;
fn failed() -> AppError {
    AppError::new(ErrorCode::ExportFailed)
}
pub(crate) fn validate(request: &ExportLogsRequest) -> Result<(), AppError> {
    request.scope.validate()?;
    request.container_id.validate()?;
    if !request.secrets_acknowledged {
        return Err(AppError::new(ErrorCode::PermissionDenied));
    }
    if request.lines.is_empty() || request.lines.len() > 20_000 {
        return Err(AppError::new(ErrorCode::InvalidLimits));
    }
    let mut bytes = request.lines.len() - 1;
    for line in &request.lines {
        if line.len() > 256 * 1024 + 256 || line.chars().any(|c| (c.is_control() && c != '\t') || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}' | '\u{2028}' | '\u{2029}')) { return Err(AppError::new(ErrorCode::InvalidResponse)); }
        bytes += line.len();
        if bytes > MAX_BYTES {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
    }
    Ok(())
}
pub(crate) fn write(path: &Path, lines: &[String]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .filter(|p| p.is_absolute())
        .ok_or_else(failed)?;
    if let Ok(metadata) = fs::symlink_metadata(path)
        && !metadata.is_file()
    {
        return Err(failed());
    }
    let mut random = [0; 16];
    getrandom::fill(&mut random).map_err(|_| failed())?;
    let temp = parent.join(format!(
        ".containerdesk-export-{}",
        random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    ));
    let mut owned_temp = false;
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|_| failed())?;
        owned_temp = true;
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                file.write_all(b"\n").map_err(|_| failed())?;
            }
            file.write_all(line.as_bytes()).map_err(|_| failed())?;
        }
        file.sync_all().map_err(|_| failed())?;
        drop(file);
        fs::rename(&temp, path).map_err(|_| failed())?;
        fs::File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| failed())?;
        Ok(())
    })();
    if result.is_err() && owned_temp {
        let _ = fs::remove_file(&temp);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn request() -> ExportLogsRequest {
        ExportLogsRequest {
            scope: crate::contract_tests::scope(),
            container_id: ContainerId("a".repeat(64)),
            lines: vec!["one čćž".into(), "[stderr / diagnostic] two".into()],
            secrets_acknowledged: true,
        }
    }
    #[test]
    fn selected_export_is_exact_private_atomic_and_cannot_follow_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "containerdesk-export-{}",
            crate::test_directory_suffix()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("selected.txt");
        let request = request();
        validate(&request).unwrap();
        write(&path, &request.lines).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), request.lines.join("\n"));
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let other = root.join("other.txt");
        fs::write(&other, b"unrelated").unwrap();
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&other, &path).unwrap();
        assert_eq!(
            write(&path, &request.lines).unwrap_err().code,
            ErrorCode::ExportFailed
        );
        assert_eq!(fs::read(&other).unwrap(), b"unrelated");
        assert!(!format!("{request:?}").contains("diagnostic] two"));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn export_requires_secret_ack_and_enforces_clean_text_byte_and_line_bounds() {
        let mut r = request();
        r.secrets_acknowledged = false;
        assert_eq!(validate(&r).unwrap_err().code, ErrorCode::PermissionDenied);
        r.secrets_acknowledged = true;
        for text in [
            "\x1b[31mred",
            "a\nb",
            "\u{009d}https://host",
            "\u{202e}hidden",
        ] {
            r.lines = vec![text.into()];
            assert!(validate(&r).is_err());
        }
        r.lines = vec!["é".repeat(130000); 33];
        assert!(validate(&r).is_err());
        r.lines = vec![String::new(); 20001];
        assert!(validate(&r).is_err());
    }
}
