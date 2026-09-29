//! Bounded transient log snapshots. Stderr can contain application logs AND transport diagnostics.
use crate::{
    domain::*,
    policy::registry::{self, ReadOperation},
    ssh::{
        multiplex::Client,
        runner::{Captured, RunError},
    },
};
use std::{collections::VecDeque, time::Duration};
pub const MAX_RECORD_BYTES: usize = 256 * 1024;
pub const MAX_RECORDS: usize = 20_000;
pub const MAX_RETAINED_BYTES: usize = 8 * 1024 * 1024;
fn err(scope: &SessionScope, code: ErrorCode) -> AppError {
    AppError::new(code).in_scope(scope)
}
fn size(record: &LogRecord) -> usize {
    record.text.len() + record.timestamp.as_ref().map_or(0, String::len)
}
fn timestamp(line: &[u8]) -> Option<&str> {
    let stamp = std::str::from_utf8(line.get(..31)?).ok()?;
    let bytes = stamp.as_bytes();
    if !stamp.is_ascii()
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'.'
        || bytes[29] != b'Z'
        || bytes[30] != b' '
    {
        return None;
    }
    if !(0..30)
        .filter(|i| ![4, 7, 10, 13, 16, 19, 29].contains(i))
        .all(|i| bytes[i].is_ascii_digit())
    {
        return None;
    }
    Some(&stamp[..30])
}
fn channel(bytes: &[u8], origin: LogChannel) -> (VecDeque<LogRecord>, u32) {
    let mut records = VecDeque::new();
    let mut retained = 0;
    let mut dropped = 0;
    let data = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if bytes.is_empty() {
        return (records, dropped);
    }
    for line in data.split(|b| *b == b'\n') {
        let stamp = timestamp(line);
        let content = if stamp.is_some() { &line[31..] } else { line };
        let invalid_utf8 = std::str::from_utf8(content).is_err();
        let bounded = &content[..content.len().min(MAX_RECORD_BYTES)];
        let mut text = String::from_utf8_lossy(bounded).into_owned();
        let mut end = text.len().min(MAX_RECORD_BYTES);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = content.len() > MAX_RECORD_BYTES || end < text.len();
        text.truncate(end);
        let record = LogRecord {
            text,
            timestamp: stamp.map(String::from),
            channel: origin.clone(),
            truncated,
            invalid_utf8,
        };
        retained += size(&record);
        records.push_back(record);
        while records.len() > MAX_RECORDS || retained > MAX_RETAINED_BYTES {
            if let Some(old) = records.pop_front() {
                retained -= size(&old);
                dropped += 1;
            }
        }
    }
    (records, dropped)
}
pub(crate) fn decode(
    request: &ContainerLogsRequest,
    output: Captured,
) -> Result<LogSnapshot, AppError> {
    let scope = &request.scope;
    if !output.status.success() {
        let unsupported = std::str::from_utf8(&output.stderr).is_ok_and(|s| {
            s.trim()
                == "Error response from daemon: configured logging driver does not support reading"
        });
        let missing = std::str::from_utf8(&output.stderr).is_ok_and(|s| {
            s.trim()
                == format!(
                    "Error response from daemon: No such container: {}",
                    request.container_id.0
                )
        });
        return Err(err(
            scope,
            if output.status.code() == Some(255) {
                ErrorCode::Disconnected
            } else if unsupported {
                ErrorCode::LogDriverUnsupported
            } else if missing {
                ErrorCode::ContainerNotFound
            } else {
                ErrorCode::TransportUnavailable
            },
        ));
    }
    if output.stdout.len() > MAX_RETAINED_BYTES || output.stderr.len() > MAX_RETAINED_BYTES {
        return Err(err(scope, ErrorCode::ResourceLimit));
    }
    let stderr_ambiguous = !output.stderr.is_empty();
    let (out, a) = channel(&output.stdout, LogChannel::Stdout);
    let (stderr, b) = channel(&output.stderr, LogChannel::StderrAmbiguous);
    let mut records: Vec<_> = out.into_iter().chain(stderr).collect();
    records.sort_by(|a, b| match (&a.timestamp, &b.timestamp) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    });
    let mut retained: usize = records.iter().map(size).sum();
    let mut remove = 0;
    while records.len() - remove > MAX_RECORDS || retained > MAX_RETAINED_BYTES {
        retained -= size(&records[remove]);
        remove += 1;
    }
    records.drain(..remove);
    let dropped_records = a + b + remove as u32;
    let truncated = dropped_records > 0 || records.iter().any(|r| r.truncated);
    Ok(LogSnapshot {
        scope: scope.clone(),
        container_id: request.container_id.clone(),
        records,
        truncated,
        dropped_records,
        stderr_ambiguous,
    })
}
pub(crate) async fn read(
    connection: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: &ContainerLogsRequest,
) -> Result<LogSnapshot, AppError> {
    request.scope.validate()?;
    let operation = ReadOperation::ContainerLogs {
        container_id: request.container_id.clone(),
        tail: request.tail,
        timeout_seconds: request.timeout_seconds,
        since: request.since.clone(),
        until: request.until.clone(),
    };
    let plan = registry::read(&operation)?;
    let deadline = Duration::from_secs(plan.timeout_seconds().into());
    let scope = &request.scope;
    tokio::time::timeout(deadline, async {
        let (fresh, _) = super::probe::run(connection, options).await;
        if fresh.daemon_id.as_deref() != Some(scope.daemon_id.as_str()) {
            return Err(err(scope, ErrorCode::StaleSession));
        }
        let command = binding
            .prepare(plan, &fresh)
            .map_err(|e| e.in_scope(scope))?;
        let output = connection
            .start_log_snapshot(command.encoded().into(), deadline)
            .map_err(|e| e.in_scope(scope))?
            .wait()
            .await
            .map_err(|e| {
                err(
                    scope,
                    match e {
                        RunError::TimedOut => ErrorCode::OperationTimedOut,
                        RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
                        RunError::Cancelled => ErrorCode::OperationCancelled,
                        _ => ErrorCode::TransportUnavailable,
                    },
                )
            })?;
        let result = decode(request, output)?;
        let (after, _) = super::probe::run(connection, options).await;
        binding
            .prepare(registry::read(&operation)?, &after)
            .map_err(|e| e.in_scope(scope))?;
        Ok(result)
    })
    .await
    .map_err(|_| err(scope, ErrorCode::OperationTimedOut))?
}
#[cfg(test)]
mod tests;
