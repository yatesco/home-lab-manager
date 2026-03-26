use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedDefaultConfig {
    pub alert: DefaultAlertName,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedFullConfig {
    pub storage: ParsedStorageConfig,
    pub alerting: BTreeMap<String, ParsedAlertingProviderConfig>,
    pub endpoints: Vec<ParsedEndpointConfig>,
    #[serde(rename = "external-endpoints")]
    pub external_endpoints: Vec<ParsedExternalEndpointConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedStorageConfig {
    #[serde(rename = "type")]
    pub storage_type: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedAlertingProviderConfig {
    #[serde(rename = "application-token")]
    pub application_token: String,
    #[serde(rename = "user-key")]
    pub user_key: String,
    #[serde(rename = "default-alert")]
    pub default_alert: ParsedDefaultAlertConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedDefaultAlertConfig {
    pub enabled: bool,
    pub description: String,
    #[serde(rename = "send-on-resolved")]
    pub send_on_resolved: bool,
    #[serde(rename = "failure-threshold")]
    pub failure_threshold: u32,
    #[serde(rename = "success-threshold")]
    pub success_threshold: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ParsedAlertConfigReference(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DefaultAlertName {
    None,
    IfMissing(ParsedAlertConfigReference),
    Always(ParsedAlertConfigReference),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedEndpointConfig {
    pub name: String,
    pub url: String,
    pub group: String,
    pub interval: String,
    pub conditions: Vec<String>,
    #[serde(default)]
    pub alerts: Vec<ParsedAlertRefConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedExternalEndpointConfig {
    pub name: String,
    pub group: String,
    pub token: String,
    pub heartbeat: ParsedHeartbeatConfig,
    #[serde(default)]
    pub alerts: Vec<ParsedAlertRefConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedHeartbeatConfig {
    pub interval: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParsedAlertRefConfig {
    #[serde(rename = "type")]
    pub alert_type: ParsedAlertConfigReference,
}

pub fn parse_full_config(yaml: &str) -> Result<ParsedFullConfig, serde_yaml::Error> {
    serde_yaml::from_str(yaml)
}

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;

    use super::{
        parse_full_config, DefaultAlertName, ParsedAlertConfigReference, ParsedAlertRefConfig,
        ParsedAlertingProviderConfig, ParsedDefaultAlertConfig, ParsedDefaultConfig,
        ParsedEndpointConfig, ParsedExternalEndpointConfig, ParsedFullConfig,
        ParsedHeartbeatConfig, ParsedStorageConfig,
    };

    fn deserialize_yaml<T>(yaml: &str) -> T
    where
        T: DeserializeOwned,
    {
        serde_yaml::from_str(yaml).expect("deserialization should succeed")
    }

    #[test]
    fn parses_example_full_yaml() {
        let full_yaml = include_str!("../resources/examples/full.yaml");
        let parsed = parse_full_config(full_yaml).expect("example full.yaml should parse");

        assert_eq!(parsed.storage.storage_type, "postgres");
        assert_eq!(parsed.endpoints.len(), 3);
        assert_eq!(parsed.endpoints[1].alerts.len(), 0);
        assert_eq!(parsed.external_endpoints.len(), 2);
    }

    #[test]
    fn deserializes_storage_config() {
        let yaml = "type: postgres\npath: postgres://user:pass@postgres:5432/db?sslmode=disable\n";
        let config: ParsedStorageConfig = deserialize_yaml(yaml);
        assert_eq!(config.storage_type, "postgres");
    }

    #[test]
    fn deserializes_default_alert_config() {
        let yaml = "enabled: true\ndescription: health check failed\nsend-on-resolved: true\nfailure-threshold: 3\nsuccess-threshold: 2\n";
        let config: ParsedDefaultAlertConfig = deserialize_yaml(yaml);
        assert!(config.enabled);
        assert_eq!(config.failure_threshold, 3);
    }

    #[test]
    fn deserializes_default_config_none() {
        let yaml = "alert: none\n";
        let config: ParsedDefaultConfig = deserialize_yaml(yaml);
        assert_eq!(config.alert, DefaultAlertName::None);
    }

    #[test]
    fn deserializes_default_config_if_missing() {
        let yaml = "alert: !if-missing my-alert\n";
        let config: ParsedDefaultConfig = deserialize_yaml(yaml);
        assert_eq!(
            config.alert,
            DefaultAlertName::IfMissing(ParsedAlertConfigReference("my-alert".to_string()))
        );
    }

    #[test]
    fn deserializes_default_config_always() {
        let yaml = "alert: !always my-alert\n";
        let config: ParsedDefaultConfig = deserialize_yaml(yaml);
        assert_eq!(
            config.alert,
            DefaultAlertName::Always(ParsedAlertConfigReference("my-alert".to_string()))
        );
    }

    #[test]
    fn deserializes_alert_ref_config() {
        let yaml = "type: my-alert\n";
        let config: ParsedAlertRefConfig = deserialize_yaml(yaml);
        assert_eq!(
            config.alert_type,
            ParsedAlertConfigReference("my-alert".to_string())
        );
    }

    #[test]
    fn deserializes_heartbeat_config() {
        let yaml = "interval: 24h\n";
        let config: ParsedHeartbeatConfig = deserialize_yaml(yaml);
        assert_eq!(config.interval, "24h");
    }

    #[test]
    fn deserializes_endpoint_config() {
        let yaml = "name: showmeacat\nurl: https://www.showmeacat.com/\ngroup: web\ninterval: 10m\nconditions:\n  - \"[STATUS] == 200\"\nalerts:\n  - type: my-alert\n";
        let config: ParsedEndpointConfig = deserialize_yaml(yaml);
        assert_eq!(config.name, "showmeacat");
        assert_eq!(config.alerts.len(), 1);
    }

    #[test]
    fn deserializes_external_endpoint_config() {
        let yaml = "name: call-me-once\ngroup: backups\ntoken: token-value\nheartbeat:\n  interval: 24h\nalerts:\n  - type: my-alert\n";
        let config: ParsedExternalEndpointConfig = deserialize_yaml(yaml);
        assert_eq!(config.token, "token-value");
        assert_eq!(config.heartbeat.interval, "24h");
    }

    #[test]
    fn deserializes_alerting_provider_config() {
        let yaml = "application-token: app-token\nuser-key: user-key\ndefault-alert:\n  enabled: true\n  description: health check failed\n  send-on-resolved: true\n  failure-threshold: 3\n  success-threshold: 2\n";
        let config: ParsedAlertingProviderConfig = deserialize_yaml(yaml);
        assert_eq!(config.application_token, "app-token");
        assert!(config.default_alert.enabled);
    }

    #[test]
    fn deserializes_full_config() {
        let yaml = "storage:\n  type: postgres\n  path: postgres://user:pass@postgres:5432/db?sslmode=disable\nalerting:\n  my-alert:\n    application-token: app-token\n    user-key: user-key\n    default-alert:\n      enabled: true\n      description: health check failed\n      send-on-resolved: true\n      failure-threshold: 3\n      success-threshold: 2\nendpoints:\n  - name: showmeacat\n    url: https://www.showmeacat.com/\n    group: web\n    interval: 10m\n    conditions:\n      - \"[STATUS] == 200\"\nexternal-endpoints:\n  - name: call-me-once\n    group: backups\n    token: token-value\n    heartbeat:\n      interval: 24h\n";
        let config: ParsedFullConfig = deserialize_yaml(yaml);
        assert_eq!(config.storage.storage_type, "postgres");
        assert_eq!(config.endpoints.len(), 1);
        assert_eq!(config.external_endpoints.len(), 1);
    }
}
