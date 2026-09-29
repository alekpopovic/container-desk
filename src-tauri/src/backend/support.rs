use super::*;
impl Backend {
    pub fn prepare_support_report(&self) -> Result<SupportPreview, AppError> {
        self.require_live_mode()?;
        let _permit = self
            .export_slot
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        use crate::support::{Source, SourceError};
        let mut source_errors = Vec::new();
        let preferences = self
            .preferences()
            .map_err(|e| {
                source_errors.push(SourceError {
                    source: Source::Preferences,
                    code: e.code,
                })
            })
            .ok();
        let connection = self
            .sessions
            .current()
            .map_err(|e| {
                source_errors.push(SourceError {
                    source: Source::Connection,
                    code: e.code,
                })
            })
            .ok()
            .flatten();
        let records = self
            .activity_records()
            .map_err(|e| {
                source_errors.push(SourceError {
                    source: Source::Activity,
                    code: e.code,
                })
            })
            .unwrap_or_default();
        let text = crate::support::report(
            preferences.as_ref().map(|p| &p.preferences),
            connection.as_ref(),
            &records,
            &source_errors,
        )?;
        self.support_preview
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .prepare(text)
    }
    pub fn clear_support_data(&self, request: ClearSupportRequest) -> Result<(), AppError> {
        self.require_live_mode()?;
        if !request.confirmed {
            return Err(AppError::new(ErrorCode::PermissionDenied));
        }
        // The save dialog owns this permit through its actual write; a clear cannot race it.
        let _permit = self
            .export_slot
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.activities.as_ref().map_err(Clone::clone)?.clear()?;
        self.support_preview
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .clear();
        Ok(())
    }
    pub async fn save_support_report(
        &self,
        window: tauri::WebviewWindow,
        request: SaveSupportRequest,
    ) -> Result<bool, AppError> {
        use tauri_plugin_dialog::DialogExt;
        self.require_live_mode()?;
        let _permit = self
            .export_slot
            .try_acquire()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        // Validate before opening UI, then revalidate after the user has chosen a location.
        self.support_preview
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .get(&request.preview_id)?;
        let (sender, receiver) = tokio::sync::oneshot::channel();
        window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Save reviewed support report")
            .set_file_name("containerdesk-support.json")
            .add_filter("Support report", &["json"])
            .save_file(move |chosen| {
                let _ = sender.send(chosen);
            });
        let chosen = receiver
            .await
            .map_err(|_| AppError::new(ErrorCode::ExportFailed))?;
        let Some(chosen) = chosen else {
            return Ok(false);
        };
        self.require_live_mode()?;
        let report = self
            .support_preview
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .get(&request.preview_id)?;
        let path = chosen
            .into_path()
            .map_err(|_| AppError::new(ErrorCode::ExportFailed))?;
        tokio::task::spawn_blocking(move || crate::log_export::write(&path, &[report]))
            .await
            .map_err(|_| AppError::new(ErrorCode::ExportFailed))??;
        self.support_preview
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?
            .clear();
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_clear_preserves_ssh_files_and_preferences_invalidates_export_and_respects_dialog() {
        use std::fs;
        let root = std::env::temp_dir().join(format!(
            "containerdesk-support-{}",
            crate::test_directory_suffix()
        ));
        let home = root.join("owned-home");
        let ssh = home.join(".ssh");
        fs::create_dir_all(&ssh).unwrap();
        for name in ["config", "id_ed25519", "known_hosts"] {
            fs::write(ssh.join(name), format!("SYNTHETIC_043_SECRET_{name}")).unwrap();
        }
        let data = root.join("app-data");
        fs::create_dir_all(data.join("activity")).unwrap();
        let history = serde_json::json!({"schemaVersion":2,"records":[{"id":format!("i_{}","1".repeat(32)),"hostId":format!("h_{}","2".repeat(32)),"action":"stop","targets":["a".repeat(64)],"startedAtMs":1000,"updatedAtMs":1234,"outcome":"failed","results":[{"containerId":"a".repeat(64),"outcome":"failed","dispatched":true,"error":"operation_timed_out"}]}]});
        fs::write(
            data.join("activity/history.json"),
            serde_json::to_vec(&history).unwrap(),
        )
        .unwrap();
        let backend = Backend::new(&data, home);
        let before = backend.preferences().unwrap();
        let preview = backend.prepare_support_report().unwrap();
        assert!(preview.report.contains("operation_timed_out"));
        assert!(!preview.report.contains("SYNTHETIC_043_SECRET"));
        assert!(!preview.report.contains(&"a".repeat(64)));
        assert_eq!(
            backend
                .clear_support_data(ClearSupportRequest { confirmed: false })
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        );
        let dialog = backend.export_slot.try_acquire().unwrap();
        assert_eq!(
            backend
                .clear_support_data(ClearSupportRequest { confirmed: true })
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        drop(dialog);
        backend
            .clear_support_data(ClearSupportRequest { confirmed: true })
            .unwrap();
        assert!(backend.activity_records().unwrap().is_empty());
        assert!(
            backend
                .support_preview
                .lock()
                .unwrap()
                .get(&preview.id)
                .is_err()
        );
        assert_eq!(backend.preferences().unwrap(), before);
        for name in ["config", "id_ed25519", "known_hosts"] {
            assert_eq!(
                fs::read_to_string(ssh.join(name)).unwrap(),
                format!("SYNTHETIC_043_SECRET_{name}")
            );
        }
        drop(backend);
        let reopened = Backend::new(&data, root.join("owned-home"));
        assert!(reopened.activity_records().unwrap().is_empty());
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }
}
