//! Read-only network snapshots. Endpoint membership is reported, never inferred from names.
use crate::{
    domain::*,
    policy::registry::{self, ReadOperation},
    ssh::{
        multiplex::Client,
        runner::{Limits, RunError},
    },
};
use serde_json::Value;
use std::{collections::BTreeMap, net::IpAddr, time::Duration};
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_ITEMS: usize = 5000;
pub(crate) const LIST_TEMPLATE: &str = r#"{"id":{{json .ID}},"name":{{json .Name}},"driver":{{json .Driver}},"scope":{{json .Scope}},"internal":{{json .Internal}},"ipv6":{{json .IPv6}}}"#;
fn err(scope: &SessionScope, code: ErrorCode) -> AppError {
    AppError::new(code).in_scope(scope)
}
fn bound(scope: &SessionScope, n: usize, max: usize) -> Result<(), AppError> {
    if n > max {
        Err(err(scope, ErrorCode::ResourceLimit))
    } else {
        Ok(())
    }
}
struct Fields<'a> {
    scope: &'a SessionScope,
    incomplete: bool,
}
impl Fields<'_> {
    fn text(&mut self, v: &Value) -> Result<Option<String>, AppError> {
        if v.is_null() {
            return Ok(None);
        }
        match v.as_str() {
            Some(value) => {
                bound(self.scope, value.len(), 4096)?;
                if value.chars().any(char::is_control) {
                    self.incomplete = true;
                    Ok(None)
                } else {
                    Ok((!value.is_empty()).then(|| value.into()))
                }
            }
            None => {
                self.incomplete = true;
                Ok(None)
            }
        }
    }
    fn boolean(&mut self, v: &Value) -> Option<bool> {
        match v {
            Value::Bool(b) => Some(*b),
            Value::String(s) if s == "true" => Some(true),
            Value::String(s) if s == "false" => Some(false),
            Value::Null => None,
            _ => {
                self.incomplete = true;
                None
            }
        }
    }
    fn address(
        &mut self,
        v: &Value,
        family: Option<bool>,
        cidr: bool,
    ) -> Result<Option<String>, AppError> {
        let value = self.text(v)?;
        if let Some(value) = value {
            let (address, prefix) = value
                .split_once('/')
                .map_or((value.as_str(), None), |(a, p)| (a, Some(p)));
            let valid = address.parse::<IpAddr>().ok().is_some_and(|ip| {
                family.is_none_or(|v6| ip.is_ipv6() == v6)
                    && match prefix {
                        Some(p) => {
                            cidr && p
                                .parse::<u8>()
                                .is_ok_and(|n| n <= if ip.is_ipv6() { 128 } else { 32 })
                        }
                        None => true,
                    }
            });
            if valid {
                Ok(Some(value))
            } else {
                self.incomplete = true;
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    fn masked(&mut self, v: &Value) -> Result<Vec<DetailValue>, AppError> {
        if v.is_null() {
            return Ok(vec![]);
        }
        let Some(map) = v.as_object() else {
            self.incomplete = true;
            return Ok(vec![]);
        };
        bound(self.scope, map.len(), 256)?;
        map.iter()
            .map(|(name, value)| {
                bound(self.scope, name.len(), 4096)?;
                if let Some(value) = value.as_str() {
                    bound(self.scope, value.len(), 65536)?;
                } else {
                    self.incomplete = true;
                }
                Ok(DetailValue {
                    name: name.clone(),
                    value: None,
                    masked: true,
                })
            })
            .collect()
    }
}
pub(crate) fn parse_list(
    scope: &SessionScope,
    bytes: &[u8],
) -> Result<ListNetworksResponse, AppError> {
    scope.validate()?;
    bound(scope, bytes.len(), MAX_BYTES)?;
    let mut rows = BTreeMap::new();
    let mut fields = Fields {
        scope,
        incomplete: false,
    };
    if !bytes.is_empty() {
        for line in bytes
            .strip_suffix(b"\n")
            .unwrap_or(bytes)
            .split(|b| *b == b'\n')
        {
            bound(scope, line.len(), 32768)?;
            let row: Value =
                serde_json::from_slice(line).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
            let id = NetworkId(
                row["id"]
                    .as_str()
                    .ok_or_else(|| err(scope, ErrorCode::InvalidResponse))?
                    .into(),
            );
            id.validate()
                .map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
            let summary = NetworkSummary {
                scope: scope.clone(),
                id: id.clone(),
                name: fields
                    .text(&row["name"])?
                    .ok_or_else(|| err(scope, ErrorCode::InvalidResponse))?,
                driver: fields.text(&row["driver"])?,
                network_scope: fields.text(&row["scope"])?,
                internal: fields.boolean(&row["internal"]),
                ipv6: fields.boolean(&row["ipv6"]),
            };
            if rows.insert(id.0, summary).is_some() {
                return Err(err(scope, ErrorCode::InvalidResponse));
            }
            bound(scope, rows.len(), MAX_ITEMS)?;
        }
    }
    Ok(ListNetworksResponse {
        scope: scope.clone(),
        networks: rows.into_values().collect(),
    })
}
pub(crate) fn parse_detail(
    request: &InspectNetworkRequest,
    bytes: &[u8],
) -> Result<NetworkDetail, AppError> {
    let scope = &request.scope;
    scope.validate()?;
    request.network_id.validate()?;
    bound(scope, bytes.len(), MAX_BYTES)?;
    let rows: Vec<Value> =
        serde_json::from_slice(bytes).map_err(|_| err(scope, ErrorCode::InvalidResponse))?;
    if rows.is_empty() {
        return Err(err(scope, ErrorCode::NetworkNotFound));
    }
    if rows.len() != 1 || rows[0]["Id"].as_str() != Some(&request.network_id.0) {
        return Err(err(scope, ErrorCode::InvalidResponse));
    }
    let row = &rows[0];
    let mut fields = Fields {
        scope,
        incomplete: false,
    };
    let summary = NetworkSummary {
        scope: scope.clone(),
        id: request.network_id.clone(),
        name: fields
            .text(&row["Name"])?
            .ok_or_else(|| err(scope, ErrorCode::InvalidResponse))?,
        driver: fields.text(&row["Driver"])?,
        network_scope: fields.text(&row["Scope"])?,
        internal: fields.boolean(&row["Internal"]),
        ipv6: fields.boolean(&row["EnableIPv6"]),
    };
    let created_at = fields.text(&row["Created"])?;
    let labels = fields.masked(&row["Labels"])?;
    let options = fields.masked(&row["Options"])?;
    let ipam = &row["IPAM"];
    if !ipam.is_null() && !ipam.is_object() {
        fields.incomplete = true;
    }
    let ipam_driver = fields.text(&ipam["Driver"])?;
    let ipam_options = fields.masked(&ipam["Options"])?;
    let mut ipam_config = vec![];
    if let Some(config) = ipam["Config"].as_array() {
        bound(scope, config.len(), 128)?;
        for entry in config {
            if !entry.is_object() {
                fields.incomplete = true;
                continue;
            }
            let mut auxiliary_addresses = vec![];
            if let Some(aux) = entry["AuxiliaryAddresses"].as_object() {
                bound(scope, aux.len(), 256)?;
                for (name, value) in aux {
                    bound(scope, name.len(), 4096)?;
                    auxiliary_addresses.push(NetworkAddress {
                        name: name.clone(),
                        address: fields.address(value, None, false)?,
                    });
                }
            } else if !entry["AuxiliaryAddresses"].is_null() {
                fields.incomplete = true;
            }
            ipam_config.push(NetworkIpamConfig {
                subnet: fields.address(&entry["Subnet"], None, true)?,
                ip_range: fields.address(&entry["IPRange"], None, true)?,
                gateway: fields.address(&entry["Gateway"], None, false)?,
                auxiliary_addresses,
            });
        }
    } else if !ipam["Config"].is_null() {
        fields.incomplete = true;
    }
    let attachments_reported = row["Containers"].is_object();
    let mut attachments = vec![];
    if let Some(containers) = row["Containers"].as_object() {
        bound(scope, containers.len(), MAX_ITEMS)?;
        for (key, endpoint) in containers {
            bound(scope, key.len(), 4096)?;
            if key.is_empty() || key.chars().any(char::is_control) {
                return Err(err(scope, ErrorCode::InvalidResponse));
            }
            let id = ContainerId(key.clone());
            if !endpoint.is_object() {
                fields.incomplete = true;
            }
            attachments.push(NetworkAttachment {
                endpoint_key: key.clone(),
                container_id: id.validate().ok().map(|()| id),
                name: fields.text(&endpoint["Name"])?,
                endpoint_id: fields.text(&endpoint["EndpointID"])?,
                ipv4_address: fields.address(&endpoint["IPv4Address"], Some(false), true)?,
                ipv6_address: fields.address(&endpoint["IPv6Address"], Some(true), true)?,
            });
        }
    } else if !row["Containers"].is_null() {
        fields.incomplete = true;
    }
    Ok(NetworkDetail {
        summary,
        created_at,
        ipam_driver,
        ipam_config,
        labels,
        options,
        ipam_options,
        attachments,
        attachments_reported,
        metadata_incomplete: fields.incomplete,
    })
}
async fn execute(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    scope: &SessionScope,
    op: ReadOperation,
) -> Result<Vec<u8>, AppError> {
    let (report, _) = super::probe::run(client, options).await;
    if report.daemon_id.as_deref() != Some(scope.daemon_id.as_str()) {
        return Err(err(scope, ErrorCode::StaleSession));
    }
    let command = binding
        .prepare(registry::read(&op)?, &report)
        .map_err(|e| e.in_scope(scope))?;
    let output = client
        .start_fixed(
            command.encoded().into(),
            Limits {
                deadline: Duration::from_secs(30),
                stdout_bytes: MAX_BYTES,
                stderr_bytes: 64 * 1024,
            },
        )?
        .wait()
        .await
        .map_err(|e| {
            err(
                scope,
                match e {
                    RunError::Cancelled => ErrorCode::Disconnected,
                    RunError::TimedOut => ErrorCode::OperationTimedOut,
                    RunError::OutputLimit(_) => ErrorCode::ResourceLimit,
                    _ => ErrorCode::TransportUnavailable,
                },
            )
        })?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    let message = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    Err(err(
        scope,
        if output.status.code() == Some(255) {
            ErrorCode::Disconnected
        } else if message.contains("permission denied") || message.contains("access denied") {
            ErrorCode::PermissionDenied
        } else if message.contains("no such network")
            || (message.contains("network") && message.contains("not found"))
        {
            ErrorCode::NetworkNotFound
        } else {
            ErrorCode::TransportUnavailable
        },
    ))
}
pub(crate) async fn list(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    scope: &SessionScope,
) -> Result<ListNetworksResponse, AppError> {
    tokio::time::timeout(Duration::from_secs(30), async {
        parse_list(
            scope,
            &execute(client, options, binding, scope, ReadOperation::ListNetworks).await?,
        )
    })
    .await
    .map_err(|_| err(scope, ErrorCode::OperationTimedOut))?
}
pub(crate) async fn inspect(
    client: &Client,
    options: &DockerOptions,
    binding: &super::probe::VerifiedDocker,
    request: &InspectNetworkRequest,
) -> Result<NetworkDetail, AppError> {
    tokio::time::timeout(Duration::from_secs(30), async {
        parse_detail(
            request,
            &execute(
                client,
                options,
                binding,
                &request.scope,
                ReadOperation::InspectNetwork {
                    network_id: request.network_id.clone(),
                },
            )
            .await?,
        )
    })
    .await
    .map_err(|_| err(&request.scope, ErrorCode::OperationTimedOut))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_tests::scope;
    fn request() -> InspectNetworkRequest {
        InspectNetworkRequest {
            scope: scope(),
            network_id: NetworkId("a".repeat(64)),
        }
    }
    fn raw() -> Value {
        serde_json::json!([{"Id":"a".repeat(64),"Name":"custom-bridge","Driver":"bridge","Internal":true,"EnableIPv6":true,"IPAM":{"Driver":"default","Config":[{"Subnet":"10.20.0.0/24","Gateway":"10.20.0.1"},{"Subnet":"fd36::/64","Gateway":"fd36::1","AuxiliaryAddresses":{"dns":"fd36::2"}}]},"Containers":{"b".repeat(64):{"Name":"attached","EndpointID":"endpoint","IPv4Address":"10.20.0.2/24","IPv6Address":"fd36::3/64"},"lb-endpoint":{"Name":"<img src=x>","IPv4Address":"","IPv6Address":""}},"Labels":{"token":"private-label"},"Options":{"password":"private-option"}}])
    }
    #[test]
    fn bridge_dual_stack_and_unknown_endpoint_identity_survive_without_secrets() {
        let detail = parse_detail(&request(), &serde_json::to_vec(&raw()).unwrap()).unwrap();
        assert_eq!(detail.summary.internal, Some(true));
        assert_eq!(detail.summary.ipv6, Some(true));
        assert_eq!(detail.ipam_config.len(), 2);
        assert_eq!(detail.ipam_config[1].gateway.as_deref(), Some("fd36::1"));
        assert_eq!(detail.attachments.len(), 2);
        assert!(detail.attachments_reported);
        assert!(!detail.metadata_incomplete);
        assert!(detail.attachments[0].container_id.is_some());
        assert!(detail.attachments[1].container_id.is_none());
        assert_eq!(
            detail.attachments[0].ipv6_address.as_deref(),
            Some("fd36::3/64")
        );
        assert!(
            detail
                .labels
                .iter()
                .chain(&detail.options)
                .all(|row| row.masked && row.value.is_none())
        );
        assert!(!serde_json::to_string(&detail).unwrap().contains("private-"));
    }
    #[test]
    fn host_none_and_missing_ipam_are_honest_while_malformed_optional_fields_are_unknown() {
        for driver in ["host", "null", "vendor/custom"] {
            let value = serde_json::json!([{"Id":request().network_id,"Name":driver,"Driver":driver,"Containers":{}}]);
            let detail = parse_detail(&request(), &serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(detail.ipam_config.is_empty());
            assert!(detail.attachments.is_empty());
            assert!(detail.attachments_reported);
            assert!(!detail.metadata_incomplete);
            assert_eq!(detail.summary.internal, None);
        }
        let mut value = raw();
        value[0]["Internal"] = serde_json::json!([true]);
        value[0]["IPAM"]["Config"][0]["Gateway"] = serde_json::json!({"malformed":1});
        value[0]["Containers"]["b".repeat(64)]["IPv6Address"] = "fd36::1/129".into();
        value[0]["Containers"]["stale"] = Value::Null;
        let detail = parse_detail(&request(), &serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(detail.metadata_incomplete);
        assert_eq!(detail.summary.internal, None);
        assert_eq!(detail.ipam_config[0].gateway, None);
        assert_eq!(detail.attachments.len(), 3);
        assert_eq!(detail.attachments[0].ipv6_address, None);
        value[0]["Containers"] = Value::Null;
        assert!(
            !parse_detail(&request(), &serde_json::to_vec(&value).unwrap())
                .unwrap()
                .attachments_reported
        );
    }
    #[test]
    fn required_identity_bounds_and_fixed_read_commands_fail_closed() {
        let row=serde_json::json!({"id":"a".repeat(64),"name":"bridge","driver":"bridge","internal":"false","ipv6":"true"}).to_string();
        let list = parse_list(&scope(), row.as_bytes()).unwrap();
        assert_eq!(list.networks[0].internal, Some(false));
        assert_eq!(list.networks[0].ipv6, Some(true));
        assert!(parse_list(&scope(), format!("{row}\n{row}").as_bytes()).is_err());
        assert_eq!(
            parse_detail(&request(), b"[]").unwrap_err().code,
            ErrorCode::NetworkNotFound
        );
        assert_eq!(
            parse_detail(&request(), &vec![b'x'; MAX_BYTES + 1])
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
        for value in ["short", "--all", "$(touch /tmp/x)"] {
            assert!(
                registry::read(&ReadOperation::InspectNetwork {
                    network_id: NetworkId(value.into())
                })
                .is_err()
            );
        }
        assert!(
            NetworkId("xbtm0v4f1lfh6b6a6a6a6a6a6".into())
                .validate()
                .is_ok()
        );
        for op in [
            ReadOperation::ListNetworks,
            ReadOperation::InspectNetwork {
                network_id: request().network_id,
            },
        ] {
            assert_eq!(
                registry::read(&op).unwrap().category(),
                &registry::OperationCategory::Read
            );
        }
    }
}
