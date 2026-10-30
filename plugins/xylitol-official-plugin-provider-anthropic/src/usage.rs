use serde_json::Value;

use crate::dto::{PromptCacheRead, XyUsage};

pub fn from_anthropic_usage(value: &Value) -> XyUsage {
    let input = value
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output = value
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_read = value
        .get("cache_read_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_write = value
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_write_1h = value
        .get("cache_creation")
        .and_then(|c| c.get("ephemeral_1h_input_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    XyUsage {
        input,
        output,
        cache_write,
        cache_write_1h,
        total_tokens: input + output + cache_read + cache_write,
        cost: None,
        ..XyUsage::default()
    }
    .with_prompt_cache_read(PromptCacheRead::Tokens(cache_read))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_cache_read_write_and_1h_split() {
        let json = serde_json::json!({
            "input_tokens": 200,
            "output_tokens": 80,
            "cache_read_input_tokens": 40,
            "cache_creation_input_tokens": 10,
            "cache_creation": { "ephemeral_5m_input_tokens": 6, "ephemeral_1h_input_tokens": 4 }
        });
        let u = from_anthropic_usage(&json);
        assert_eq!(u.input, 200);
        assert_eq!(u.output, 80);
        assert_eq!(u.cache_read, 40);
        assert_eq!(u.cache_write, 10);
        assert_eq!(u.cache_write_1h, 4);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(40));
        assert_eq!(u.total_tokens, 330, "context total must include cache");
    }

    #[test]
    fn absent_cache_fields_report_zero_tokens() {
        let json = serde_json::json!({"input_tokens": 3, "output_tokens": 4});
        let u = from_anthropic_usage(&json);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(0));
        assert_eq!(u.total_tokens, 7);
    }
}
