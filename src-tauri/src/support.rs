//! Allowlisted support data. Never serialize source DTOs, exception messages or payloads.
use crate::{activity::ActivityRecord, domain::*};
use serde_json::json;
use std::time::{Duration, Instant};
pub const MAX_BYTES: usize = 64 * 1024;
pub const PREVIEW_LIFETIME: Duration = Duration::from_secs(300);
#[derive(Default)]
pub struct PreviewStore(Option<(SupportPreview, Instant)>);
impl PreviewStore {
    pub fn prepare(&mut self, report: String) -> Result<SupportPreview, AppError> {
        if report.len() > MAX_BYTES {
            return Err(AppError::new(ErrorCode::ResourceLimit));
        }
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| AppError::new(ErrorCode::Internal))?;
        let preview = SupportPreview {
            id: random.iter().map(|b| format!("{b:02x}")).collect(),
            report,
            expires_in_seconds: 300,
        };
        self.0 = Some((preview.clone(), Instant::now()));
        Ok(preview)
    }
    pub fn get(&mut self, id: &str) -> Result<String, AppError> {
        if self
            .0
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() >= PREVIEW_LIFETIME)
        {
            self.clear();
            return Err(AppError::new(ErrorCode::IntentExpired));
        }
        self.0
            .as_ref()
            .filter(|(p, _)| p.id == id && id.len() == 32)
            .map(|(p, _)| p.report.clone())
            .ok_or_else(|| AppError::new(ErrorCode::InvalidIntent))
    }
    pub fn clear(&mut self) {
        self.0 = None;
    }
}
fn bucket(ms: u64, limit: u64) -> u64 {
    ms.min(limit) / 100 * 100
}
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Preferences,
    Connection,
    Activity,
}
#[derive(serde::Serialize)]
pub struct SourceError {
    pub source: Source,
    pub code: ErrorCode,
}
pub fn report(
    preferences: Option<&Preferences>,
    connection: Option<&ConnectionSnapshot>,
    records: &[ActivityRecord],
    source_errors: &[SourceError],
) -> Result<String, AppError> {
    // Per-report ordinal pseudonyms have no reversible source map in the exported file.
    let mut pseudonyms = std::collections::HashMap::<HostId, String>::new();
    let mut pseudonym = |id: &HostId| {
        let next = format!("host-{}", pseudonyms.len() + 1);
        pseudonyms.entry(id.clone()).or_insert(next).clone()
    };
    let current = connection.map(|c| json!({
        "host": c.host_id.as_ref().map(&mut pseudonym),
        "state": c.state, "transportMode": c.transport_mode, "hasJump": c.has_jump,
        "config": if c.selection.use_default_config { "system-defaults" } else { "path-2" },
        "error": c.diagnostic.as_ref().map(|d| json!({"stage":d.stage,"code":d.code})),
        "timings": c.durations.iter().take(3).map(|d| json!({"stage":d.stage,"durationMsBucket":bucket(d.duration_ms.into(),30_000)})).collect::<Vec<_>>()
    }));
    let activity: Vec<_> = records.iter().rev().take(20).map(|r| json!({
        "host": pseudonym(&r.host_id), "action":r.action, "outcome":r.outcome,
        "targetCount":r.targets.len().min(20),
        "durationMsBucket":bucket(r.updated_at_ms.saturating_sub(r.started_at_ms),3_600_000),
        "errors":r.results.iter().take(20).filter_map(|r| r.error.as_ref()).collect::<Vec<_>>()
    })).collect();
    let report = json!({
        "schemaVersion":1, "appVersion":env!("CARGO_PKG_VERSION"),
        "platform":std::env::consts::OS, "architecture":std::env::consts::ARCH,
        "transport":{"kind":"native_openssh","connectionPolicy":"app_owned_multiplex_with_direct_fallback",
            "executable":match preferences { Some(p) if p.ssh_executable_override.is_some() => "path-1", Some(_) => "system-openssh", None => "unavailable" }},
        "connection":current, "activity":activity, "sourceErrors":source_errors.iter().take(3).collect::<Vec<_>>(),
        "retention":{"activityMaxRecords":200,"activityMaxBytes":524288,"exportedActivityMaxRecords":20,"previewMaxBytes":MAX_BYTES,"previewValidSeconds":300},
        "privacy":"Only app-controlled enums, counts and bucketed durations. Host/path pseudonyms are local to this report. No user-supplied text, addresses, IDs, config, environment, logs, terminal or inspect data. Review before sharing; versions, outcomes and counts can still reveal operational context."
    });
    let text =
        serde_json::to_string_pretty(&report).map_err(|_| AppError::new(ErrorCode::Internal))?;
    if text.len() > MAX_BYTES {
        return Err(AppError::new(ErrorCode::ResourceLimit));
    }
    Ok(text)
}
#[cfg(test)]
mod tests;
