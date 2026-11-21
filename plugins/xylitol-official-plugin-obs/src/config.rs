use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use xylitol_plugin_sdk::config_section::{ConfigSection, SectionBinding};

impl SectionBinding for OtelConfig {
    const KEY: &'static str = "otel";

    fn schema_value() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(OtelConfig)).expect("otel schema")
    }

    fn default_value() -> serde_json::Value {
        serde_json::to_value(OtelConfig::default()).expect("otel default")
    }
}

pub fn config_section() -> ConfigSection {
    <OtelConfig as SectionBinding>::descriptor()
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default)]
pub struct OtelConfig {
    #[serde(default)]
    pub exporter: OtelExporterKind,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub protocol: OtelHttpProtocol,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub observation_io: OtelObservationIo,
    #[serde(default)]
    pub tool_observation_io: OtelObservationIo,
    #[serde(default = "default_otel_export_timeout_secs")]
    pub export_timeout_secs: u64,
}

fn default_otel_export_timeout_secs() -> u64 {
    60
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            exporter: OtelExporterKind::default(),
            endpoint: None,
            protocol: OtelHttpProtocol::default(),
            environment: None,
            service_name: None,
            headers: HashMap::new(),
            observation_io: OtelObservationIo::default(),
            tool_observation_io: OtelObservationIo::default(),
            export_timeout_secs: default_otel_export_timeout_secs(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelObservationIo {
    #[default]
    None,
    Truncated,
    Full,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelExporterKind {
    #[default]
    None,
    OtlpHttp,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelHttpProtocol {
    HttpBinary,
    #[default]
    HttpJson,
}

impl OtelConfig {
    pub fn wants_otlp_http(&self) -> bool {
        matches!(self.exporter, OtelExporterKind::OtlpHttp)
    }
}

#[cfg(test)]
mod section_decl_tests {
    use super::*;

    #[test]
    fn otel_section_declares_key_schema_default() {
        assert_eq!(<OtelConfig as SectionBinding>::KEY, "otel");
        let section = config_section();
        assert_eq!(section.key, <OtelConfig as SectionBinding>::KEY);
        assert!((section.schema)().is_object());
        assert_eq!(
            (section.default)(),
            serde_json::to_value(OtelConfig::default()).expect("otel default")
        );
    }
}
