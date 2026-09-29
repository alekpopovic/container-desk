//! POSIX remote-shell encoding. Local SSH argv does not protect the remote command string.
use crate::domain::{AppError, ErrorCode};
pub const MAX_ARGUMENT_BYTES: usize = 16 * 1024;
pub const MAX_COMMAND_BYTES: usize = 128 * 1024;
pub fn single_quote(value: &str) -> Result<String, AppError> {
    if value.contains('\0') {
        return Err(AppError::new(ErrorCode::InvalidRemoteArgument));
    }
    if value.len() > MAX_ARGUMENT_BYTES {
        return Err(AppError::new(ErrorCode::ResourceLimit));
    }
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}
pub(crate) fn command(arguments: &[String]) -> Result<String, AppError> {
    if arguments.is_empty() || arguments.len() > 128 {
        return Err(AppError::new(ErrorCode::InvalidLimits));
    }
    let mut output = String::from("exec");
    for argument in arguments {
        let quoted = single_quote(argument)?;
        if output.len() + 1 + quoted.len() > MAX_COMMAND_BYTES {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        output.push(' ');
        output.push_str(&quoted);
    }
    Ok(output)
}
/// Only fixed Compose plans provide this validated remote context. No local shell is spawned.
pub(crate) fn in_directory(
    arguments: &[String],
    directory: &str,
    files: &[String],
) -> Result<String, AppError> {
    crate::docker::validate_absolute_path(directory)?;
    let mut output = format!("cd {}", single_quote(directory)?);
    for file in files {
        crate::docker::validate_absolute_path(file)?;
        let file = single_quote(file)?;
        output.push_str(&format!(" && test -f {file} && test -r {file}"));
    }
    output.push_str(" && ");
    output.push_str(&command(arguments)?);
    if output.len() > MAX_COMMAND_BYTES {
        return Err(AppError::new(ErrorCode::ResourceLimit));
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hostile_strings_round_trip_through_an_inert_posix_shell_without_execution() {
        let marker =
            std::env::temp_dir().join(format!("containerdesk-quote-marker-{}", std::process::id()));
        assert!(!marker.exists());
        let values = vec![
            String::new(),
            "plain".into(),
            "single'quote".into(),
            "many ''' quotes".into(),
            "white space\tand\nnewlines".into(),
            "--leading-option".into(),
            "; exit 9".into(),
            "{{json .}}".into(),
            "unicode-čćž".into(),
            format!("$(touch {})", marker.display()),
            format!("`touch {}`", marker.display()),
            format!("label'; touch {}; #", marker.display()),
            "\\\\\"double\"*?[abc]~$HOME".into(),
        ];
        let mut args = vec!["/usr/bin/printf".into(), "%s\\0".into()];
        args.extend(values.clone());
        let result = std::process::Command::new("/bin/sh")
            .args(["-c", &command(&args).unwrap()])
            .output()
            .unwrap();
        assert!(result.status.success());
        assert!(result.stdout.ends_with(&[0]));
        let actual: Vec<_> = result.stdout[..result.stdout.len() - 1]
            .split(|b| *b == 0)
            .map(|b| std::str::from_utf8(b).unwrap())
            .collect();
        assert_eq!(actual, values);
        assert!(result.stdout.ends_with(&[0]));
        assert!(!marker.exists());
    }
    #[test]
    fn compose_directory_and_file_paths_with_apostrophes_are_inert_and_access_checked() {
        let root = std::env::temp_dir().join(format!(
            "containerdesk-compose-quote-{}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let directory = root.join("project's directory");
        std::fs::create_dir(&directory).unwrap();
        let file = directory.join("config's file.yml");
        std::fs::write(&file, b"fixture").unwrap();
        let files = vec![file.to_str().unwrap().to_string()];
        let marker = root.join("unexpected");
        let args = vec![
            "/usr/bin/printf".into(),
            "%s".into(),
            format!("$(touch {})", marker.display()),
        ];
        let command = in_directory(&args, directory.to_str().unwrap(), &files).unwrap();
        let output = std::process::Command::new("/bin/sh")
            .args(["-c", &command])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, args[2].as_bytes());
        assert!(!marker.exists());
        std::fs::remove_file(&file).unwrap();
        assert!(
            !std::process::Command::new("/bin/sh")
                .args(["-c", &command])
                .output()
                .unwrap()
                .status
                .success()
        );
        std::fs::remove_dir(&directory).unwrap();
        std::fs::remove_dir(&root).unwrap();
    }
    #[test]
    fn nul_and_excessive_expansion_are_rejected() {
        assert_eq!(
            single_quote("x\0y").unwrap_err().code,
            ErrorCode::InvalidRemoteArgument
        );
        assert_eq!(
            single_quote(&"x".repeat(MAX_ARGUMENT_BYTES + 1))
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        assert_eq!(
            command(&vec!["'".repeat(MAX_ARGUMENT_BYTES); 3])
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
    }
}
