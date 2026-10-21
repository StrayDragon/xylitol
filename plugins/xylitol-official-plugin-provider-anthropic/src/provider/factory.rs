use std::sync::Arc;

use crate::hooks::HttpHooks;
use crate::wire_policy::WirePolicy;

use super::{AdapterRef, AnthropicMessagesAdapter};

pub fn build_adapter(
    api_key: String,
    model: String,
    base_url: Option<String>,
    hooks: Option<Arc<dyn HttpHooks>>,
) -> AdapterRef {
    build_adapter_with_wire_policy(api_key, model, base_url, hooks, WirePolicy::default())
}

pub fn build_adapter_with_wire_policy(
    api_key: String,
    model: String,
    base_url: Option<String>,
    hooks: Option<Arc<dyn HttpHooks>>,
    wire_policy: WirePolicy,
) -> AdapterRef {
    Arc::new(AnthropicMessagesAdapter::with_wire_policy(
        api_key,
        model,
        base_url,
        hooks,
        wire_policy,
    ))
}
