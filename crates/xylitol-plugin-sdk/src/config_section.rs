use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub struct ConfigSection {
    pub key: &'static str,
    pub schema: fn() -> Value,
    pub default: fn() -> Value,
}

impl PartialEq for ConfigSection {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for ConfigSection {}

pub trait SectionBinding: Sized + serde::de::DeserializeOwned {
    const KEY: &'static str;

    fn schema_value() -> Value;

    fn default_value() -> Value;

    fn descriptor() -> ConfigSection {
        let schema: fn() -> Value = Self::schema_value;
        let default: fn() -> Value = Self::default_value;
        ConfigSection {
            key: Self::KEY,
            schema,
            default,
        }
    }

    fn from_sections(map: &BTreeMap<String, Value>) -> Result<Self, SectionBindingError> {
        let value = map
            .get(Self::KEY)
            .ok_or(SectionBindingError::Missing(Self::KEY))?;
        serde_json::from_value(value.clone()).map_err(|e| SectionBindingError::Parse(Self::KEY, e))
    }

    fn try_from_sections(
        map: &BTreeMap<String, Value>,
    ) -> Result<Option<Self>, SectionBindingError> {
        match map.get(Self::KEY) {
            None => Ok(None),
            Some(value) => serde_json::from_value(value.clone())
                .map(Some)
                .map_err(|e| SectionBindingError::Parse(Self::KEY, e)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SectionBindingError {
    #[error("config section `{0}` is missing")]
    Missing(&'static str),
    #[error("config section `{0}`: {1}")]
    Parse(&'static str, #[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Value {
        Value::Null
    }

    fn default_value() -> Value {
        Value::Bool(true)
    }

    #[test]
    fn sections_compare_by_key() {
        let a = ConfigSection {
            key: "models",
            schema,
            default: default_value,
        };
        let b = ConfigSection {
            key: "models",
            schema: default_value,
            default: schema,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn function_pointers_are_callable() {
        let section = ConfigSection {
            key: "models",
            schema,
            default: default_value,
        };
        assert!((section.schema)().is_null());
        assert!((section.default)().is_boolean());
    }
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
    #[serde(default)]
    struct SampleSection {
        enabled: bool,
        fuel: u32,
    }

    impl SectionBinding for SampleSection {
        const KEY: &'static str = "sample";

        fn schema_value() -> Value {
            serde_json::to_value(schemars::schema_for!(Self)).expect("sample schema")
        }

        fn default_value() -> Value {
            serde_json::to_value(Self::default()).expect("sample default")
        }
    }

    fn map_with(key: &str, value: Value) -> BTreeMap<String, Value> {
        BTreeMap::from([(key.to_string(), value)])
    }

    #[test]
    fn descriptor_is_derived_from_binding() {
        let section = SampleSection::descriptor();
        assert_eq!(section.key, "sample");
        assert_eq!((section.schema)(), SampleSection::schema_value());
        assert_eq!((section.default)(), SampleSection::default_value());
    }

    #[test]
    fn from_sections_parses_present_key() {
        let map = map_with("sample", serde_json::json!({"enabled": true, "fuel": 7}));
        let parsed = SampleSection::from_sections(&map).expect("present");
        assert!(parsed.enabled);
        assert_eq!(parsed.fuel, 7);
    }

    #[test]
    fn from_sections_reports_missing_key() {
        let err = SampleSection::from_sections(&BTreeMap::new()).unwrap_err();
        assert!(matches!(err, SectionBindingError::Missing("sample")));
    }

    #[test]
    fn from_sections_reports_parse_failure() {
        let map = map_with("sample", serde_json::json!({"fuel": "seven"}));
        let err = SampleSection::from_sections(&map).unwrap_err();
        assert!(matches!(err, SectionBindingError::Parse("sample", _)));
    }

    #[test]
    fn try_from_sections_tolerates_missing_key() {
        let absent = SampleSection::try_from_sections(&BTreeMap::new()).expect("absent");
        assert_eq!(absent, None);
        let map = map_with("sample", serde_json::json!({"fuel": 1}));
        let present = SampleSection::try_from_sections(&map).expect("present");
        assert_eq!(present.map(|section| section.fuel), Some(1));
    }

    #[test]
    fn try_from_sections_still_loud_on_parse_failure() {
        let map = map_with("sample", serde_json::json!({"fuel": "seven"}));
        let err = SampleSection::try_from_sections(&map).unwrap_err();
        assert!(matches!(err, SectionBindingError::Parse("sample", _)));
    }
}
