//! Normalize vendor usage JSON into [`AiBridgeUsage`].

use serde_json::Value;

use crate::dto::AiBridgeUsage;
use crate::wire_policy::WirePolicy;

/// OpenAI-style `{prompt_tokens, completion_tokens, ...}` → [`AiBridgeUsage`].
pub fn from_openai_usage(value: &Value) -> AiBridgeUsage {
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
        cache_read: value
            .get("prompt_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_write: 0,
        cache_write_1h: 0,
        total_tokens: total,
        cost: None,
    }
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
        cache_read,
        cache_write,
        cache_write_1h,
        total_tokens: input + output,
        cost: None,
    }
}

/// OpenAI Responses `{input_tokens, output_tokens}` → [`AiBridgeUsage`].
///
/// Uses [`WirePolicy::default()`] (cache_read stays 0 until policy expects it).
pub fn from_responses_usage(value: &Value) -> AiBridgeUsage {
    from_responses_usage_with_policy(value, WirePolicy::default())
}

/// Responses usage mapping gated by [`WirePolicy`] (c1880).
///
/// When `!expects_prompt_cache_usage()`, `cache_read` is forced to 0 (do not
/// pretend first-language cache fields). When true, map
/// `input_tokens_details.cached_tokens` if present.
pub fn from_responses_usage_with_policy(value: &Value, policy: WirePolicy) -> AiBridgeUsage {
    let input = value
        .get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output = value
        .get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_read = if policy.expects_prompt_cache_usage() {
        value
            .get("input_tokens_details")
            .and_then(|d| d.get("cached_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    } else {
        0
    };
    AiBridgeUsage {
        input,
        output,
        cache_read,
        cache_write: 0,
        cache_write_1h: 0,
        total_tokens: input + output,
        cost: None,
    }
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
        assert_eq!(u.cache_read, 0);
    }

    #[test]
    fn responses_usage_ignores_cached_tokens_when_policy_off() {
        let json = serde_json::json!({
            "input_tokens": 30,
            "output_tokens": 20,
            "input_tokens_details": { "cached_tokens": 12 }
        });
        let u = from_responses_usage_with_policy(&json, WirePolicy::default());
        assert_eq!(u.cache_read, 0);
    }

    #[test]
    fn responses_usage_maps_cached_tokens_when_policy_on() {
        let policy = WirePolicy {
            compat: crate::wire_policy::Compat::Generic,
            extra_policy: crate::wire_policy::ExtraPolicy {
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
        assert_eq!(u.cache_read, 12);
    }

    #[test]
    fn apply_cost_rates_fills_cost() {
        let mut u = AiBridgeUsage {
            input: 1_000_000,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            cache_write_1h: 0,
            total_tokens: 1_000_000,
            cost: None,
        };
        apply_cost_rates(&mut u, 10.0, 30.0, 1.0, 5.0);
        let cost = u.cost.unwrap();
        assert!((cost.input - 10.0).abs() < 1e-6);
    }
}
