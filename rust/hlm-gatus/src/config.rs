use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::incoming::{
    ParsedAlertConfigReference, ParsedAlertRefConfig, ParsedAlertingProviderConfig,
    ParsedDefaultAlertConfig, ParsedDefaultConfig, ParsedEndpointConfig,
    ParsedExternalEndpointConfig, ParsedFullConfig, ParsedHeartbeatConfig, ParsedStorageConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompleteFullConfig {
    pub default: Config,
    pub full: FullConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigBuildError {
    UndefinedAlertReference { reference: String },
}

impl CompleteFullConfig {
    pub fn from_parsed(
        parsed_default: ParsedDefaultConfig,
        parsed_full: ParsedFullConfig,
    ) -> Result<Self, ConfigBuildError> {
        let default: Config = parsed_default.into();
        Ok(Self {
            full: FullConfig::from_parsed_with_default(parsed_full, &default.alert)?,
            default,
        })
    }
}

impl std::fmt::Display for ConfigBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndefinedAlertReference { reference } => {
                write!(f, "undefined alert reference: {reference}")
            }
        }
    }
}

impl std::error::Error for ConfigBuildError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub alert: AlertName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FullConfig {
    pub storage: StorageConfig,
    pub alerting: BTreeMap<String, AlertingProviderConfig>,
    pub endpoints: Vec<EndpointConfig>,
    #[serde(rename = "external-endpoints")]
    pub external_endpoints: Vec<ExternalEndpointConfig>,
}

impl FullConfig {
    pub fn alert_reference_from_string(
        &self,
        reference: String,
    ) -> Result<AlertConfigReference, ConfigBuildError> {
        if self.alerting.contains_key(&reference) {
            Ok(AlertConfigReference(reference))
        } else {
            Err(ConfigBuildError::UndefinedAlertReference { reference })
        }
    }

    fn from_parsed_with_default(
        value: ParsedFullConfig,
        default_alert: &AlertName,
    ) -> Result<Self, ConfigBuildError> {
        let mut full = Self {
            storage: value.storage.into(),
            alerting: value
                .alerting
                .into_iter()
                .map(|(name, provider)| (name, provider.into()))
                .collect(),
            endpoints: value.endpoints.into_iter().map(Into::into).collect(),
            external_endpoints: value
                .external_endpoints
                .into_iter()
                .map(Into::into)
                .collect(),
        };
        full.apply_default_to_endpoints(default_alert)?;
        full.validate_alert_references()?;
        Ok(full)
    }

    fn apply_default_to_endpoints(
        &mut self,
        default_alert: &AlertName,
    ) -> Result<(), ConfigBuildError> {
        match default_alert {
            AlertName::None => {}
            AlertName::IfMissing(reference) => {
                let validated_reference = self.alert_reference_from_string(reference.0.clone())?;
                for endpoint in &mut self.endpoints {
                    if endpoint.alerts.is_empty() {
                        endpoint.alerts.push(AlertRefConfig {
                            alert_type: validated_reference.clone(),
                        });
                    }
                }
            }
            AlertName::Always(reference) => {
                let validated_reference = self.alert_reference_from_string(reference.0.clone())?;
                for endpoint in &mut self.endpoints {
                    let has_default = endpoint
                        .alerts
                        .iter()
                        .any(|alert| alert.alert_type == validated_reference);
                    if !has_default {
                        endpoint.alerts.push(AlertRefConfig {
                            alert_type: validated_reference.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_alert_references(&self) -> Result<(), ConfigBuildError> {
        for endpoint in &self.endpoints {
            for alert in &endpoint.alerts {
                self.alert_reference_from_string(alert.alert_type.0.clone())?;
            }
        }

        for endpoint in &self.external_endpoints {
            for alert in &endpoint.alerts {
                self.alert_reference_from_string(alert.alert_type.0.clone())?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(rename = "type")]
    pub storage_type: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertingProviderConfig {
    #[serde(rename = "application-token")]
    pub application_token: String,
    #[serde(rename = "user-key")]
    pub user_key: String,
    #[serde(rename = "default-alert")]
    pub default_alert: AlertConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    pub description: String,
    #[serde(rename = "send-on-resolved")]
    pub send_on_resolved: bool,
    #[serde(rename = "failure-threshold")]
    pub failure_threshold: u32,
    #[serde(rename = "success-threshold")]
    pub success_threshold: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AlertConfigReference(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlertName {
    None,
    IfMissing(AlertConfigReference),
    Always(AlertConfigReference),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointConfig {
    pub name: String,
    pub url: String,
    pub group: String,
    pub interval: String,
    pub conditions: Vec<String>,
    #[serde(default)]
    pub alerts: Vec<AlertRefConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalEndpointConfig {
    pub name: String,
    pub group: String,
    pub token: String,
    pub heartbeat: HeartbeatConfig,
    #[serde(default)]
    pub alerts: Vec<AlertRefConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    pub interval: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertRefConfig {
    #[serde(rename = "type")]
    pub alert_type: AlertConfigReference,
}

impl From<ParsedDefaultConfig> for Config {
    fn from(value: ParsedDefaultConfig) -> Self {
        Self {
            alert: value.alert.into(),
        }
    }
}

impl From<ParsedFullConfig> for FullConfig {
    fn from(value: ParsedFullConfig) -> Self {
        Self {
            storage: value.storage.into(),
            alerting: value
                .alerting
                .into_iter()
                .map(|(name, provider)| (name, provider.into()))
                .collect(),
            endpoints: value.endpoints.into_iter().map(Into::into).collect(),
            external_endpoints: value
                .external_endpoints
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<ParsedStorageConfig> for StorageConfig {
    fn from(value: ParsedStorageConfig) -> Self {
        Self {
            storage_type: value.storage_type,
            path: value.path,
        }
    }
}

impl From<ParsedAlertingProviderConfig> for AlertingProviderConfig {
    fn from(value: ParsedAlertingProviderConfig) -> Self {
        Self {
            application_token: value.application_token,
            user_key: value.user_key,
            default_alert: value.default_alert.into(),
        }
    }
}

impl From<ParsedDefaultAlertConfig> for AlertConfig {
    fn from(value: ParsedDefaultAlertConfig) -> Self {
        Self {
            enabled: value.enabled,
            description: value.description,
            send_on_resolved: value.send_on_resolved,
            failure_threshold: value.failure_threshold,
            success_threshold: value.success_threshold,
        }
    }
}

impl From<ParsedAlertConfigReference> for AlertConfigReference {
    fn from(value: ParsedAlertConfigReference) -> Self {
        Self(value.0)
    }
}

impl From<crate::incoming::DefaultAlertName> for AlertName {
    fn from(value: crate::incoming::DefaultAlertName) -> Self {
        match value {
            crate::incoming::DefaultAlertName::None => Self::None,
            crate::incoming::DefaultAlertName::IfMissing(reference) => {
                Self::IfMissing(reference.into())
            }
            crate::incoming::DefaultAlertName::Always(reference) => Self::Always(reference.into()),
        }
    }
}

impl From<ParsedEndpointConfig> for EndpointConfig {
    fn from(value: ParsedEndpointConfig) -> Self {
        Self {
            name: value.name,
            url: value.url,
            group: value.group,
            interval: value.interval,
            conditions: value.conditions,
            alerts: value.alerts.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ParsedExternalEndpointConfig> for ExternalEndpointConfig {
    fn from(value: ParsedExternalEndpointConfig) -> Self {
        Self {
            name: value.name,
            group: value.group,
            token: value.token,
            heartbeat: value.heartbeat.into(),
            alerts: value.alerts.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ParsedHeartbeatConfig> for HeartbeatConfig {
    fn from(value: ParsedHeartbeatConfig) -> Self {
        Self {
            interval: value.interval,
        }
    }
}

impl From<ParsedAlertRefConfig> for AlertRefConfig {
    fn from(value: ParsedAlertRefConfig) -> Self {
        Self {
            alert_type: value.alert_type.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::incoming::{parse_full_config, ParsedDefaultConfig};

    use super::{AlertConfigReference, AlertName, CompleteFullConfig, ConfigBuildError};

    #[test]
    fn builds_complete_config_from_parsed_inputs() {
        let parsed_default: ParsedDefaultConfig =
            serde_yaml::from_str("alert: !if-missing my-alert\n").expect("default should parse");
        let parsed_full = parse_full_config(include_str!("../resources/examples/full.yaml"))
            .expect("full config should parse");

        let complete = CompleteFullConfig::from_parsed(parsed_default, parsed_full)
            .expect("build should pass");

        assert_eq!(complete.full.endpoints.len(), 3);
        assert_eq!(
            complete.default.alert,
            AlertName::IfMissing(AlertConfigReference("my-alert".to_string()))
        );
        assert_eq!(complete.full.endpoints[0].alerts.len(), 1);
        assert_eq!(complete.full.endpoints[1].alerts.len(), 1);
        assert_eq!(
            complete.full.endpoints[1].alerts[0].alert_type,
            AlertConfigReference("my-alert".to_string())
        );
        assert_eq!(complete.full.endpoints[2].alerts.len(), 1);
    }

    #[test]
    fn keeps_endpoint_alerts_empty_when_default_is_none() {
        let parsed_default: ParsedDefaultConfig =
            serde_yaml::from_str("alert: none\n").expect("default should parse");
        let parsed_full = parse_full_config(include_str!("../resources/examples/full.yaml"))
            .expect("full config should parse");

        let complete = CompleteFullConfig::from_parsed(parsed_default, parsed_full)
            .expect("build should pass");

        assert_eq!(complete.full.endpoints[1].alerts.len(), 0);
    }

    #[test]
    fn applies_always_default_without_duplicate() {
        let parsed_default: ParsedDefaultConfig =
            serde_yaml::from_str("alert: !always my-alert\n").expect("default should parse");
        let parsed_full = parse_full_config(
            "storage:\n  type: postgres\n  path: postgres://user:pass@postgres:5432/db?sslmode=disable\nalerting:\n  my-alert:\n    application-token: app-token\n    user-key: user-key\n    default-alert:\n      enabled: true\n      description: health check failed\n      send-on-resolved: true\n      failure-threshold: 3\n      success-threshold: 2\n  something-else:\n    application-token: app-token\n    user-key: user-key\n    default-alert:\n      enabled: true\n      description: health check failed\n      send-on-resolved: true\n      failure-threshold: 3\n      success-threshold: 2\nendpoints:\n  - name: no-alerts\n    url: https://example.com/no-alerts\n    group: web\n    interval: 10m\n    conditions:\n      - \"[STATUS] == 200\"\n  - name: other-alert\n    url: https://example.com/other\n    group: web\n    interval: 10m\n    conditions:\n      - \"[STATUS] == 200\"\n    alerts:\n      - type: something-else\n  - name: already-has-default\n    url: https://example.com/default\n    group: web\n    interval: 10m\n    conditions:\n      - \"[STATUS] == 200\"\n    alerts:\n      - type: my-alert\nexternal-endpoints: []\n",
        )
        .expect("full config should parse");

        let complete = CompleteFullConfig::from_parsed(parsed_default, parsed_full)
            .expect("build should pass");

        assert_eq!(complete.full.endpoints[0].alerts.len(), 1);
        assert_eq!(
            complete.full.endpoints[0].alerts[0].alert_type,
            AlertConfigReference("my-alert".to_string())
        );

        assert_eq!(complete.full.endpoints[1].alerts.len(), 2);
        assert_eq!(
            complete.full.endpoints[1].alerts[0].alert_type,
            AlertConfigReference("something-else".to_string())
        );
        assert_eq!(
            complete.full.endpoints[1].alerts[1].alert_type,
            AlertConfigReference("my-alert".to_string())
        );

        assert_eq!(complete.full.endpoints[2].alerts.len(), 1);
        assert_eq!(
            complete.full.endpoints[2].alerts[0].alert_type,
            AlertConfigReference("my-alert".to_string())
        );
    }

    #[test]
    fn serializes_complete_config_to_toml() {
        let parsed_default: ParsedDefaultConfig =
            serde_yaml::from_str("alert: none\n").expect("default should parse");
        let parsed_full = parse_full_config(include_str!("../resources/examples/full.yaml"))
            .expect("full config should parse");
        let complete = CompleteFullConfig::from_parsed(parsed_default, parsed_full)
            .expect("build should pass");

        let toml = toml::to_string(&complete).expect("complete config should serialize to TOML");

        assert!(toml.contains("[full.storage]"));
        assert!(toml.contains("[full.alerting.my-alert]"));
        assert!(toml.contains("[[full.endpoints]]"));
        assert!(toml.contains("[[full.external-endpoints]]"));
    }

    #[test]
    fn fails_when_endpoint_alert_reference_is_undefined() {
        let parsed_default: ParsedDefaultConfig =
            serde_yaml::from_str("alert: none\n").expect("default should parse");
        let parsed_full = parse_full_config(
            "storage:\n  type: postgres\n  path: postgres://user:pass@postgres:5432/db?sslmode=disable\nalerting:\n  my-alert:\n    application-token: app-token\n    user-key: user-key\n    default-alert:\n      enabled: true\n      description: health check failed\n      send-on-resolved: true\n      failure-threshold: 3\n      success-threshold: 2\nendpoints:\n  - name: missing-ref\n    url: https://example.com/missing-ref\n    group: web\n    interval: 10m\n    conditions:\n      - \"[STATUS] == 200\"\n    alerts:\n      - type: does-not-exist\nexternal-endpoints: []\n",
        )
        .expect("full config should parse");

        let error = CompleteFullConfig::from_parsed(parsed_default, parsed_full)
            .expect_err("build should fail for undefined alert references");

        assert_eq!(
            error,
            ConfigBuildError::UndefinedAlertReference {
                reference: "does-not-exist".to_string(),
            }
        );
    }

    #[test]
    fn fails_when_default_alert_reference_is_undefined() {
        let parsed_default: ParsedDefaultConfig =
            serde_yaml::from_str("alert: !if-missing does-not-exist\n")
                .expect("default should parse");
        let parsed_full = parse_full_config(include_str!("../resources/examples/full.yaml"))
            .expect("full config should parse");

        let error = CompleteFullConfig::from_parsed(parsed_default, parsed_full)
            .expect_err("build should fail for undefined default alert reference");

        assert_eq!(
            error,
            ConfigBuildError::UndefinedAlertReference {
                reference: "does-not-exist".to_string(),
            }
        );
    }
}
