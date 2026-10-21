use std::collections::HashMap;

pub trait XySecretResolver: Send + Sync + std::fmt::Debug {
    fn resolve_config_value(
        &self,
        config: &str,
        env: Option<&HashMap<String, String>>,
    ) -> Option<String>;

    fn resolve_headers(
        &self,
        headers: &HashMap<String, String>,
        env: Option<&HashMap<String, String>>,
    ) -> Option<HashMap<String, String>>;
}
