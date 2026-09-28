//! Native SSH configuration boundaries. Discovery never invokes a subprocess.
pub mod auth;
pub mod discovery;
pub mod quoting;
pub mod resolver;
pub mod runner;
pub mod sessions;
use crate::domain::{AppError, ErrorCode};

pub fn validate_alias(alias: &str) -> Result<(), AppError> {
    let bytes = alias.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 256
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(b))
    {
        return Err(AppError::new(ErrorCode::InvalidAlias));
    }
    Ok(())
}
