use std::collections::BTreeMap;
use std::process::Command;

use serde::Deserialize;

use crate::config::FullConfig;
use crate::incoming::{
    ParsedAlertConfigReference, ParsedAlertRefConfig, ParsedAlertingProviderConfig,
    ParsedDefaultAlertConfig, ParsedEndpointConfig, ParsedExternalEndpointConfig,
    ParsedFullConfig, ParsedHeartbeatConfig, ParsedStorageConfig,
};

pub const LABEL_PREFIX: &str = "hlm-gatus.endpoint.";
pub const LABEL_ENABLED: &str = "hlm-gatus.endpoint.enabled";
pub const LABEL_NAME: &str = "hlm-gatus.endpoint.name";
pub const LABEL_URL: &str = "hlm-gatus.endpoint.url";
pub const LABEL_GROUP: &str = "hlm-gatus.endpoint.group";
pub const LABEL_INTERVAL: &str = "hlm-gatus.endpoint.interval";
pub const LABEL_CONDITIONS: &str = "hlm-gatus.endpoint.conditions";
pub const LABEL_ALERTS: &str = "hlm-gatus.endpoint.alerts";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerConfigError {
    DockerCommandFailed { command: String, stderr: String },
    DockerInspectParse { container_id: String, error: String },
    MissingRequiredLabel { container: String, label: String },
    InvalidBooleanLabel { container: String, label: String, value: String },
    UndefinedAlertReference { reference: String },
}

impl std::fmt::Display for DockerConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DockerCommandFailed { command, stderr } => {
                write!(f, "docker command failed: {command}; {stderr}")
            }
            Self::DockerInspectParse {
                container_id,
                error,
            } => {
                write!(f, "failed to parse docker inspect for {container_id}: {error}")
            }
            Self::MissingRequiredLabel { container, label } => {
                write!(f, "missing required label {label} for container {container}")
            }
            Self::InvalidBooleanLabel {
                container,
                label,
                value,
            } => {
                write!(f, "invalid boolean label {label}={value} for container {container}")
            }
            Self::UndefinedAlertReference { reference } => {
                write!(f, "undefined alert reference: {reference}")
            }
        }
    }
}

impl std::error::Error for DockerConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningContainer {
    pub id: String,
    pub name: String,
    pub labels: BTreeMap<String, String>,
}

pub trait DockerLabelSource {
    fn running_containers(&self) -> Result<Vec<RunningContainer>, DockerConfigError>;
}

pub struct DockerCliLabelSource;

impl DockerLabelSource for DockerCliLabelSource {
    fn running_containers(&self) -> Result<Vec<RunningContainer>, DockerConfigError> {
        let ids_output = run_docker(&["ps", "--format", "{{.ID}}"])?;
        let ids: Vec<&str> = ids_output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();

        let mut containers = Vec::new();
        for id in ids {
            let inspect_output = run_docker(&["inspect", id, "--format", "{{json .}}"])?;
            let inspect: DockerInspect = serde_json::from_str(inspect_output.trim()).map_err(|e| {
                DockerConfigError::DockerInspectParse {
                    container_id: id.to_string(),
                    error: e.to_string(),
                }
            })?;
            let labels = inspect
                .config
                .and_then(|c| c.labels)
                .unwrap_or_default()
                .into_iter()
                .collect::<BTreeMap<_, _>>();

            containers.push(RunningContainer {
                id: id.to_string(),
                name: inspect.name.trim_start_matches('/').to_string(),
                labels,
            });
        }

        Ok(containers)
    }
}

pub fn build_parsed_from_running_docker(
    full: &FullConfig,
) -> Result<ParsedFullConfig, DockerConfigError> {
    build_parsed_from_label_source(full, &DockerCliLabelSource)
}

pub fn build_parsed_from_label_source<S: DockerLabelSource>(
    full: &FullConfig,
    source: &S,
) -> Result<ParsedFullConfig, DockerConfigError> {
    let containers = source.running_containers()?;
    let mut endpoints = Vec::new();

    for container in containers {
        if let Some(endpoint) = endpoint_from_container(full, &container)? {
            endpoints.push(endpoint);
        }
    }

    Ok(ParsedFullConfig {
        storage: ParsedStorageConfig {
            storage_type: full.storage.storage_type.clone(),
            path: full.storage.path.clone(),
        },
        alerting: full
            .alerting
            .iter()
            .map(|(name, provider)| {
                (
                    name.clone(),
                    ParsedAlertingProviderConfig {
                        application_token: provider.application_token.clone(),
                        user_key: provider.user_key.clone(),
                        default_alert: ParsedDefaultAlertConfig {
                            enabled: provider.default_alert.enabled,
                            description: provider.default_alert.description.clone(),
                            send_on_resolved: provider.default_alert.send_on_resolved,
                            failure_threshold: provider.default_alert.failure_threshold,
                            success_threshold: provider.default_alert.success_threshold,
                        },
                    },
                )
            })
            .collect(),
        endpoints,
        external_endpoints: full
            .external_endpoints
            .iter()
            .map(|external| ParsedExternalEndpointConfig {
                name: external.name.clone(),
                group: external.group.clone(),
                token: external.token.clone(),
                heartbeat: ParsedHeartbeatConfig {
                    interval: external.heartbeat.interval.clone(),
                },
                alerts: external
                    .alerts
                    .iter()
                    .map(|alert| ParsedAlertRefConfig {
                        alert_type: ParsedAlertConfigReference(alert.alert_type.0.clone()),
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn endpoint_from_container(
    full: &FullConfig,
    container: &RunningContainer,
) -> Result<Option<ParsedEndpointConfig>, DockerConfigError> {
    let enabled = parse_enabled(container)?;
    if !enabled {
        return Ok(None);
    }

    let url = label_required(container, LABEL_URL)?;
    let name = label_optional(container, LABEL_NAME)
        .unwrap_or_else(|| container.name.clone());
    let group = label_optional(container, LABEL_GROUP).unwrap_or_else(|| "docker".to_string());
    let interval =
        label_optional(container, LABEL_INTERVAL).unwrap_or_else(|| "60s".to_string());
    let conditions = label_optional(container, LABEL_CONDITIONS)
        .map(|v| {
            v.split("||")
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["[STATUS] == 200".to_string()]);

    let alerts = label_optional(container, LABEL_ALERTS)
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let parsed_alerts = alerts
        .into_iter()
        .map(|alert_name| {
            full.alert_reference_from_string(alert_name.clone())
                .map_err(|_| DockerConfigError::UndefinedAlertReference {
                    reference: alert_name.clone(),
                })?;
            Ok(ParsedAlertRefConfig {
                alert_type: ParsedAlertConfigReference(alert_name),
            })
        })
        .collect::<Result<Vec<_>, DockerConfigError>>()?;

    Ok(Some(ParsedEndpointConfig {
        name,
        url,
        group,
        interval,
        conditions,
        alerts: parsed_alerts,
    }))
}

fn parse_enabled(container: &RunningContainer) -> Result<bool, DockerConfigError> {
    let has_endpoint_labels = container
        .labels
        .keys()
        .any(|key| key.starts_with(LABEL_PREFIX));

    let enabled_label = label_optional(container, LABEL_ENABLED);
    match enabled_label.as_deref() {
        None => Ok(has_endpoint_labels && label_optional(container, LABEL_URL).is_some()),
        Some("1") | Some("true") | Some("yes") => Ok(true),
        Some("0") | Some("false") | Some("no") => Ok(false),
        Some(value) => Err(DockerConfigError::InvalidBooleanLabel {
            container: container.name.clone(),
            label: LABEL_ENABLED.to_string(),
            value: value.to_string(),
        }),
    }
}

fn label_optional(container: &RunningContainer, key: &str) -> Option<String> {
    container.labels.get(key).cloned()
}

fn label_required(container: &RunningContainer, key: &str) -> Result<String, DockerConfigError> {
    container
        .labels
        .get(key)
        .cloned()
        .ok_or_else(|| DockerConfigError::MissingRequiredLabel {
            container: container.name.clone(),
            label: key.to_string(),
        })
}

fn run_docker(args: &[&str]) -> Result<String, DockerConfigError> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .map_err(|e| DockerConfigError::DockerCommandFailed {
            command: format!("docker {}", args.join(" ")),
            stderr: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(DockerConfigError::DockerCommandFailed {
            command: format!("docker {}", args.join(" ")),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[derive(Debug, Deserialize)]
struct DockerInspect {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Config")]
    config: Option<DockerInspectConfig>,
}

#[derive(Debug, Deserialize)]
struct DockerInspectConfig {
    #[serde(rename = "Labels")]
    labels: Option<BTreeMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AlertConfig, AlertConfigReference, AlertRefConfig, AlertingProviderConfig,
        ExternalEndpointConfig, FullConfig, HeartbeatConfig, StorageConfig,
    };

    struct FakeDockerSource {
        containers: Vec<RunningContainer>,
    }

    impl DockerLabelSource for FakeDockerSource {
        fn running_containers(&self) -> Result<Vec<RunningContainer>, DockerConfigError> {
            Ok(self.containers.clone())
        }
    }

    fn sample_full_config() -> FullConfig {
        let mut alerting = BTreeMap::new();
        alerting.insert(
            "my-alert".to_string(),
            AlertingProviderConfig {
                application_token: "app-token".to_string(),
                user_key: "user-key".to_string(),
                default_alert: AlertConfig {
                    enabled: true,
                    description: "health check failed".to_string(),
                    send_on_resolved: true,
                    failure_threshold: 3,
                    success_threshold: 2,
                },
            },
        );

        FullConfig {
            storage: StorageConfig {
                storage_type: "postgres".to_string(),
                path: "postgres://user:pass@postgres:5432/db?sslmode=disable".to_string(),
            },
            alerting,
            endpoints: vec![],
            external_endpoints: vec![ExternalEndpointConfig {
                name: "external".to_string(),
                group: "backups".to_string(),
                token: "token".to_string(),
                heartbeat: HeartbeatConfig {
                    interval: "24h".to_string(),
                },
                alerts: vec![AlertRefConfig {
                    alert_type: AlertConfigReference("my-alert".to_string()),
                }],
            }],
        }
    }

    #[test]
    fn builds_parsed_endpoints_from_container_labels() {
        let full = sample_full_config();
        let source = FakeDockerSource {
            containers: vec![RunningContainer {
                id: "abc123".to_string(),
                name: "svc-web".to_string(),
                labels: BTreeMap::from([
                    (LABEL_ENABLED.to_string(), "true".to_string()),
                    (LABEL_URL.to_string(), "https://example.com/health".to_string()),
                    (LABEL_GROUP.to_string(), "web".to_string()),
                    (LABEL_INTERVAL.to_string(), "10m".to_string()),
                    (
                        LABEL_CONDITIONS.to_string(),
                        "[STATUS] == 200||[RESPONSE_TIME] < 300".to_string(),
                    ),
                    (LABEL_ALERTS.to_string(), "my-alert".to_string()),
                ]),
            }],
        };

        let parsed = build_parsed_from_label_source(&full, &source).expect("build should pass");

        assert_eq!(parsed.endpoints.len(), 1);
        assert_eq!(parsed.endpoints[0].name, "svc-web");
        assert_eq!(parsed.endpoints[0].group, "web");
        assert_eq!(parsed.endpoints[0].conditions.len(), 2);
        assert_eq!(parsed.endpoints[0].alerts.len(), 1);
        assert_eq!(parsed.external_endpoints.len(), 1);
    }

    #[test]
    fn skips_non_opted_in_containers() {
        let full = sample_full_config();
        let source = FakeDockerSource {
            containers: vec![RunningContainer {
                id: "abc123".to_string(),
                name: "svc-web".to_string(),
                labels: BTreeMap::new(),
            }],
        };

        let parsed = build_parsed_from_label_source(&full, &source).expect("build should pass");

        assert_eq!(parsed.endpoints.len(), 0);
    }

    #[test]
    fn fails_for_undefined_alert_reference() {
        let full = sample_full_config();
        let source = FakeDockerSource {
            containers: vec![RunningContainer {
                id: "abc123".to_string(),
                name: "svc-web".to_string(),
                labels: BTreeMap::from([
                    (LABEL_ENABLED.to_string(), "true".to_string()),
                    (LABEL_URL.to_string(), "https://example.com/health".to_string()),
                    (LABEL_ALERTS.to_string(), "missing-alert".to_string()),
                ]),
            }],
        };

        let error = build_parsed_from_label_source(&full, &source)
            .expect_err("build should fail for undefined alert references");
        assert_eq!(
            error,
            DockerConfigError::UndefinedAlertReference {
                reference: "missing-alert".to_string(),
            }
        );
    }
}