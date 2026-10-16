//! Normalize vendor usage JSON into [`AiBridgeUsage`].

use serde_json::Value;

use crate::dto::{AiBridgeUsage, PromptCacheRead};
use crate::wire_policy::WirePolicy;

/// OpenAI Completions `{prompt_tokens, completion_tokens, ...}` → [`AiBridgeUsage`].
///
/// Uses [`WirePolicy::default()`] (expects prompt-cache usage by default).
pub fn from_openai_usage(value: &Value) -> AiBridgeUsage {
    from_openai_usage_with_policy(value, WirePolicy::default())
}

/// Completions usage mapping gated by [`WirePolicy`].
///
/// Cache read: `prompt_cache_hit_tokens` first, else `prompt_tokens_details.cached_tokens`.
///
/// - `!expects_prompt_cache_usage()` → [`PromptCacheRead::NotApplicable`]
/// - expects + a cache field present → [`PromptCacheRead::Tokens`] (including 0)
/// - expects + both absent → [`PromptCacheRead::NotReported`]
pub fn from_openai_usage_with_policy(value: &Value, policy: WirePolicy) -> AiBridgeUsage {
    let input = value
        .get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output = value
        .get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let total = value
        .get("total_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(input + output);
    AiBridgeUsage {
        input,
        output,
        cache_write: 0,
        cache_write_1h: 0,
        total_tokens: total,
        cost: None,
        ..AiBridgeUsage::default()
    }
    .with_prompt_cache_read(completions_prompt_cache_read(value, policy))
}

fn completions_prompt_cache_read(value: &Value, policy: WirePolicy) -> PromptCacheRead {
    if !policy.expects_prompt_cache_usage() {
        return PromptCacheRead::NotApplicable;
    }
    if let Some(n) = value
        .get("prompt_cache_hit_tokens")
        .and_then(|v| v.as_u64())
    {
        return PromptCacheRead::Tokens(n);
    }
    if let Some(n) = value
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_u64())
    {
        return PromptCacheRead::Tokens(n);
    }
    PromptCacheRead::NotReported
}

/// Anthropic-style `{input_tokens, output_tokens, cache_*}` → [`AiBridgeUsage`].
pub fn from_anthropic_usage(value: &Value) -> AiBridgeUsage {
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
    AiBridgeUsage {
        input,
        output,
        cache_write,
        cache_write_1h,
        total_tokens: input + output,
        cost: None,
        ..AiBridgeUsage::default()
    }
    .with_prompt_cache_read(PromptCacheRead::Tokens(cache_read))
}

/// OpenAI Responses `{input_tokens, output_tokens}` → [`AiBridgeUsage`].
///
/// Uses [`WirePolicy::default()`] (expects prompt-cache usage by default, c1885).
pub fn from_responses_usage(value: &Value) -> AiBridgeUsage {
    from_responses_usage_with_policy(value, WirePolicy::default())
}

/// Responses usage mapping gated by [`WirePolicy`] (c1880 / c1885).
///
/// - `!expects_prompt_cache_usage()` → [`PromptCacheRead::NotApplicable`]
/// - expects + `input_tokens_details.cached_tokens` present → [`PromptCacheRead::Tokens`]
/// - expects + field absent → [`PromptCacheRead::NotReported`]
pub fn from_responses_usage_with_policy(value: &Value, policy: WirePolicy) -> AiBridgeUsage {
    let input = value
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output = value
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let prompt_cache_read = if !policy.expects_prompt_cache_usage() {
        PromptCacheRead::NotApplicable
    } else {
        match value
            .get("input_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64())
        {
            Some(n) => PromptCacheRead::Tokens(n),
            None => PromptCacheRead::NotReported,
        }
    };
    AiBridgeUsage {
        input,
        output,
        cache_write: 0,
        cache_write_1h: 0,
        total_tokens: input + output,
        cost: None,
        ..AiBridgeUsage::default()
    }
    .with_prompt_cache_read(prompt_cache_read)
}

/// Optional cost fill from per-million token rates.
pub fn apply_cost_rates(
    usage: &mut AiBridgeUsage,
    per_m_input: f64,
    per_m_output: f64,
    per_m_cache_read: f64,
    per_m_cache_write: f64,
) {
    usage.compute_cost(
        per_m_input,
        per_m_output,
        per_m_cache_read,
        per_m_cache_write,
    );
}

pub fn total_context_tokens(usage: &AiBridgeUsage) -> u64 {
    if usage.total_tokens > 0 {
        return usage.total_tokens;
    }
    usage.input + usage.output + usage.cache_read + usage.cache_write
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire_policy::{Compat, ExtraPolicy};

    #[test]
    fn openai_usage_maps_prompt_and_completion() {
        let json = serde_json::json!({
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "total_tokens": 150
        });
        let u = from_openai_usage(&json);
        assert_eq!(u.input, 100);
        assert_eq!(u.output, 50);
        assert_eq!(u.total_tokens, 150);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::NotReported);
        assert_eq!(u.cache_read, 0);
    }

    #[test]
    fn openai_usage_prefers_prompt_cache_hit_tokens() {
        let json = serde_json::json!({
            "prompt_tokens": 754,
            "completion_tokens": 10,
            "total_tokens": 764,
            "prompt_cache_hit_tokens": 640,
            "prompt_tokens_details": { "cached_tokens": 1 }
        });
        let u = from_openai_usage_with_policy(&json, WirePolicy::for_compat(Compat::Deepseek));
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(640));
        assert_eq!(u.cache_read, 640);
    }

    #[test]
    fn openai_usage_falls_back_to_prompt_tokens_details() {
        let json = serde_json::json!({
            "prompt_tokens": 754,
            "completion_tokens": 10,
            "total_tokens": 764,
            "prompt_tokens_details": { "cached_tokens": 640 }
        });
        let u = from_openai_usage_with_policy(&json, WirePolicy::for_compat(Compat::Deepseek));
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(640));
        assert_eq!(u.cache_read, 640);
    }

    #[test]
    fn openai_usage_not_applicable_when_policy_off() {
        let policy = WirePolicy {
            compat: Compat::Generic,
            extra_policy: ExtraPolicy {
                prompt_cache_usage: false,
                prompt_cache_key: false,
                previous_response_id: false,
            },
        };
        let json = serde_json::json!({
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "prompt_cache_hit_tokens": 40,
            "prompt_tokens_details": { "cached_tokens": 40 }
        });
        let u = from_openai_usage_with_policy(&json, policy);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::NotApplicable);
        assert_eq!(u.cache_read, 0);
    }

    #[test]
    fn openai_usage_tokens_zero_is_not_not_reported() {
        let json = serde_json::json!({
            "prompt_tokens": 100,
            "completion_tokens": 50,
            "prompt_cache_hit_tokens": 0
        });
        let u = from_openai_usage(&json);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(0));
    }

    #[test]
    fn deepseek_policy_maps_responses_cached_tokens() {
        let json = serde_json::json!({
            "input_tokens": 754,
            "output_tokens": 20,
            "input_tokens_details": { "cached_tokens": 640 }
        });
        let u = from_responses_usage_with_policy(&json, WirePolicy::for_compat(Compat::Deepseek));
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(640));
        assert_eq!(u.cache_read, 640);
    }

    #[test]
    fn anthropic_usage_maps_cache_fields() {
        let json = serde_json::json!({
            "input_tokens": 200,
            "output_tokens": 80,
            "cache_read_input_tokens": 40,
            "cache_creation_input_tokens": 10
        });
        let u = from_anthropic_usage(&json);
        assert_eq!(u.input, 200);
        assert_eq!(u.output, 80);
        assert_eq!(u.cache_read, 40);
        assert_eq!(u.cache_write, 10);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(40));
    }

    #[test]
    fn responses_usage_maps_input_output() {
        let json = serde_json::json!({
            "input_tokens": 30,
            "output_tokens": 20
        });
        let u = from_responses_usage(&json);
        assert_eq!(u.input, 30);
        assert_eq!(u.output, 20);
        assert_eq!(u.total_tokens, 50);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::NotReported);
        assert_eq!(u.cache_read, 0);
    }

    #[test]
    fn responses_usage_not_applicable_when_policy_off() {
        let policy = WirePolicy {
            compat: Compat::Generic,
            extra_policy: ExtraPolicy {
                prompt_cache_usage: false,
                prompt_cache_key: false,
                previous_response_id: false,
            },
        };
        let json = serde_json::json!({
            "input_tokens": 30,
            "output_tokens": 20,
            "input_tokens_details": { "cached_tokens": 12 }
        });
        let u = from_responses_usage_with_policy(&json, policy);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::NotApplicable);
        assert_eq!(u.cache_read, 0);
    }

    #[test]
    fn responses_usage_maps_cached_tokens_when_policy_on() {
        let policy = WirePolicy {
            compat: Compat::Generic,
            extra_policy: ExtraPolicy {
                prompt_cache_usage: true,
                prompt_cache_key: false,
                previous_response_id: false,
            },
        };
        let json = serde_json::json!({
            "input_tokens": 30,
            "output_tokens": 20,
            "input_tokens_details": { "cached_tokens": 12 }
        });
        let u = from_responses_usage_with_policy(&json, policy);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(12));
        assert_eq!(u.cache_read, 12);
    }

    #[test]
    fn responses_usage_tokens_zero_is_not_not_reported() {
        let json = serde_json::json!({
            "input_tokens": 30,
            "output_tokens": 20,
            "input_tokens_details": { "cached_tokens": 0 }
        });
        let u = from_responses_usage(&json);
        assert_eq!(u.prompt_cache_read, PromptCacheRead::Tokens(0));
    }

    #[test]
    fn default_policy_expects_prompt_cache_usage() {
        assert!(WirePolicy::default().expects_prompt_cache_usage());
    }

    #[test]
    fn apply_cost_rates_fills_cost() {
        let mut u = AiBridgeUsage {
            input: 1_000_000,
            output: 0,
            total_tokens: 1_000_000,
            ..AiBridgeUsage::default()
        };
        apply_cost_rates(&mut u, 10.0, 30.0, 1.0, 5.0);
        let cost = u.cost.unwrap();
        assert!((cost.input - 10.0).abs() < 1e-6);
    }
}
