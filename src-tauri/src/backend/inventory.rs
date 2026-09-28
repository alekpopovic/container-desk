//! Application metadata only: no SSH config edits, remote deletion or startup connections.
use super::*;
impl Backend {
    fn inventory_mode(&self, mode: &WorkspaceMode) -> Result<(), AppError> {
        if self.shutting_down.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(AppError::new(ErrorCode::Disconnected));
        }
        if self.workspace_mode()?.mode != *mode {
            return Err(AppError::new(ErrorCode::StaleSession));
        }
        Ok(())
    }
    fn inventory_saved(&self, mode: &WorkspaceMode) -> Result<PreferencesSnapshot, AppError> {
        if *mode == WorkspaceMode::Demo {
            Ok(PreferencesSnapshot {
                preferences: self
                    .demo_inventory
                    .lock()
                    .map_err(|_| AppError::new(ErrorCode::Internal))?
                    .clone(),
                notice: None,
                writable: true,
            })
        } else {
            self.preferences()
        }
    }
    fn replace_inventory(
        &self,
        mode: &WorkspaceMode,
        mut preferences: Preferences,
        revision: u32,
    ) -> Result<(), AppError> {
        if *mode == WorkspaceMode::Demo {
            let mut saved = self
                .demo_inventory
                .lock()
                .map_err(|_| AppError::new(ErrorCode::Internal))?;
            if saved.revision != revision {
                return Err(AppError::new(ErrorCode::StorageConflict));
            }
            crate::storage::validate(&preferences)?;
            preferences.revision = revision
                .checked_add(1)
                .ok_or_else(|| AppError::new(ErrorCode::ResourceLimit))?;
            *saved = preferences;
        } else {
            self.settings
                .lock()
                .map_err(|_| AppError::new(ErrorCode::Internal))?
                .as_mut()
                .map_err(|error| error.clone())?
                .replace(preferences, revision)?;
        }
        Ok(())
    }
    pub fn host_inventory(&self, request: InventoryModeRequest) -> Result<HostInventory, AppError> {
        self.inventory_mode(&request.mode)?;
        let saved = self.inventory_saved(&request.mode)?;
        let connection = self.sessions.current()?.filter(|c| {
            c.host_id.as_ref().is_some_and(|id| {
                saved.preferences.hosts.iter().any(|h| {
                    &h.id == id
                        && h.alias == c.selection.alias
                        && h.docker == c.docker_options
                        && h.ssh.as_ref().is_none_or(|ssh| ssh == &c.selection)
                })
            })
        });
        Ok(HostInventory {
            mode: request.mode,
            saved,
            connection,
        })
    }
    pub async fn save_host(&self, request: SaveHostRequest) -> Result<HostInventory, AppError> {
        let _control = self
            .inventory_control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.inventory_mode(&request.mode)?;
        let mut preferences = self.inventory_saved(&request.mode)?.preferences;
        if preferences.revision != request.expected_revision {
            return Err(AppError::new(ErrorCode::StorageConflict));
        }
        crate::ssh::resolver::arguments(&request.draft.ssh)?;
        crate::docker::DockerCommandConfig::from_options(&request.draft.docker)?;
        if request.mode == WorkspaceMode::Live
            && request.draft.ssh.use_default_config
            && request.draft.ssh.config_path
                != crate::ssh::discovery::config_path(&self.config_home, None)?
        {
            return Err(AppError::new(ErrorCode::InvalidConfigPath));
        }
        let id = if let Some(id) = request.id {
            id.validate()?;
            if !preferences.hosts.iter().any(|h| h.id == id) {
                return Err(AppError::new(ErrorCode::HostNotFound));
            }
            id
        } else {
            let mut random = [0; 16];
            getrandom::fill(&mut random).map_err(|_| AppError::new(ErrorCode::Internal))?;
            HostId(format!(
                "h_{}",
                random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ))
        };
        let old = preferences.hosts.iter().find(|h| h.id == id);
        let changed_target = old.is_some_and(|host| {
            host.ssh.as_ref() != Some(&request.draft.ssh) || host.docker != request.draft.docker
        });
        let host = SavedHost {
            id: id.clone(),
            alias: request.draft.ssh.alias.clone(),
            display_name: request.draft.display_name,
            group: request.draft.group,
            labels: request.draft.labels,
            favorite: request.draft.favorite,
            read_only: old.map(|h| h.read_only).unwrap_or(true),
            ssh: Some(request.draft.ssh),
            docker: request.draft.docker,
        };
        if let Some(index) = preferences.hosts.iter().position(|h| h.id == id) {
            preferences.hosts[index] = host;
        } else {
            preferences.hosts.push(host);
        }
        preferences.selected_host_id = Some(id.clone());
        crate::storage::validate(&preferences)?;
        if changed_target {
            self.stop_inventory_host(&id).await?;
        }
        self.replace_inventory(&request.mode, preferences, request.expected_revision)?;
        self.host_inventory(InventoryModeRequest { mode: request.mode })
    }
    async fn stop_inventory_host(&self, id: &HostId) -> Result<(), AppError> {
        if let Some(current) = self.sessions.current()?
            && current.host_id.as_ref() == Some(id)
        {
            self.sessions.disconnect(&current.token).await?;
        }
        Ok(())
    }
    pub async fn remove_host(&self, request: RemoveHostRequest) -> Result<HostInventory, AppError> {
        let _control = self
            .inventory_control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.inventory_mode(&request.mode)?;
        request.host_id.validate()?;
        let mut preferences = self.inventory_saved(&request.mode)?.preferences;
        if preferences.revision != request.expected_revision {
            return Err(AppError::new(ErrorCode::StorageConflict));
        }
        let index = preferences
            .hosts
            .iter()
            .position(|h| h.id == request.host_id)
            .ok_or_else(|| AppError::new(ErrorCode::HostNotFound))?;
        self.stop_inventory_host(&request.host_id).await?;
        preferences.hosts.remove(index);
        if preferences.selected_host_id.as_ref() == Some(&request.host_id) {
            preferences.selected_host_id = None;
        }
        self.replace_inventory(&request.mode, preferences, request.expected_revision)?;
        self.host_inventory(InventoryModeRequest { mode: request.mode })
    }
    pub async fn connect_inventory_host(
        &self,
        request: InventoryConnectRequest,
    ) -> Result<HostInventory, AppError> {
        let _control = self
            .inventory_control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.inventory_mode(&request.mode)?;
        request.host_id.validate()?;
        let preferences = self.inventory_saved(&request.mode)?.preferences;
        let host = preferences
            .hosts
            .iter()
            .find(|h| h.id == request.host_id)
            .ok_or_else(|| AppError::new(ErrorCode::HostNotFound))?;
        let selection = match &host.ssh {
            Some(ssh) => ssh.clone(),
            None => self.select_alias(SelectSshAliasRequest {
                alias: host.alias.clone(),
                config_path: preferences.trusted_config_path.clone(),
            })?,
        };
        let mode = request.mode.clone();
        self.sessions
            .begin_owned(
                selection,
                host.docker.clone(),
                Some(host.id.clone()),
                || {
                    let permit = self
                        .diagnostic_slot
                        .clone()
                        .try_acquire_owned()
                        .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
                    self.inventory_mode(&mode)?;
                    if mode == WorkspaceMode::Demo {
                        Ok(Arc::new(DemoDriver {
                            _permit: permit,
                            docker: host.docker.clone(),
                        })
                            as Arc<dyn crate::ssh::sessions::StageDriver>)
                    } else {
                        Ok(Arc::new(crate::ssh::sessions::NativeDriver {
                            runner: self.process_runner.clone(),
                            executable: preferences
                                .ssh_executable_override
                                .clone()
                                .unwrap_or_else(|| crate::diagnostics::DEFAULT_SSH.into()),
                            _permit: permit,
                            connection: Default::default(),
                            docker_options: host.docker.clone(),
                            docker_binding: Default::default(),
                        }))
                    }
                },
            )
            .await?;
        self.host_inventory(InventoryModeRequest { mode: request.mode })
    }
    pub async fn disconnect_inventory_host(
        &self,
        request: InventoryDisconnectRequest,
    ) -> Result<HostInventory, AppError> {
        let _control = self
            .inventory_control
            .try_lock()
            .map_err(|_| AppError::new(ErrorCode::ResourceLimit))?;
        self.inventory_mode(&request.mode)?;
        let current = self.sessions.snapshot(&request.token)?;
        if current.host_id.as_ref() != Some(&request.host_id) {
            return Err(AppError::new(ErrorCode::StaleSession));
        }
        self.sessions.disconnect(&request.token).await?;
        self.host_inventory(InventoryModeRequest { mode: request.mode })
    }
}
struct DemoDriver {
    _permit: tokio::sync::OwnedSemaphorePermit,
    docker: DockerOptions,
}
impl crate::ssh::sessions::StageDriver for DemoDriver {
    fn run<'a>(
        &'a self,
        stage: ConnectionStage,
        selected: &'a SshSelection,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::ssh::sessions::StageOutcome> + Send + 'a>,
    > {
        use crate::ssh::sessions::StageOutcome;
        Box::pin(async move {
            match stage {
                ConnectionStage::Resolve => StageOutcome::Resolved {
                    has_jump: selected.alias.contains("jump"),
                    effective: Some(Box::new(EffectiveSshConfig {
                        selection: selected.clone(),
                        executable_path: "/demo/ssh".into(),
                        hostname: "192.0.2.10".into(),
                        user: "demo".into(),
                        port: 22,
                        proxy_jump: selected
                            .alias
                            .contains("jump")
                            .then(|| "demo-bastion".into()),
                        has_proxy_command: false,
                    })),
                },
                ConnectionStage::Authenticate => {
                    StageOutcome::Authenticated(SshTransportMode::Unconnected)
                }
                ConnectionStage::Probe => StageOutcome::Probed(Box::new(DockerProbeReport {
                    status: DockerProbeStatus::Ready,
                    context: Some(self.docker.context.clone().unwrap_or_else(|| "demo".into())),
                    endpoint: Some("unix:///demo/docker.sock".into()),
                    endpoint_kind: Some(DockerEndpointKind::Unix),
                    client_version: Some("demo".into()),
                    server_version: Some("demo".into()),
                    daemon_id: Some("demo-inventory-daemon".into()),
                    os: Some("linux".into()),
                    rootless: Some(false),
                    compose: ComposeAvailability::Available,
                    compose_version: Some("demo".into()),
                    sudo: self.docker.sudo,
                })),
            }
        })
    }
    fn quiesce(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
}
#[cfg(test)]
mod tests;
