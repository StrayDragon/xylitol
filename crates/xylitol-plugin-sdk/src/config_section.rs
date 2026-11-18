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
