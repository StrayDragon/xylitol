//! Provider attribution — provider-specific HTTP headers and display names.
//!
//! When making LLM API requests, certain providers (OpenRouter, NVIDIA NIM,
//! Cloudflare, Vercel AI Gateway, OpenCode) require or prefer attribution
//! headers for analytics, billing, or session tracking.

use std::collections::HashMap;

// ── Provider Display Names ────────────────────────────────────────

/// Built-in provider display name lookup table (27+ providers).
/// Maps provider API identifiers to human-readable names.
#[allow(dead_code)]
pub const BUILT_IN_PROVIDER_DISPLAY_NAMES: &[(&str, &str)] = &[
    ("anthropic", "Anthropic"),
    ("amazon-bedrock", "Amazon Bedrock"),
    ("ant-ling", "Ant Ling"),
    ("azure-openai-responses", "Azure OpenAI Responses"),
    ("cerebras", "Cerebras"),
    ("cloudflare-ai-gateway", "Cloudflare AI Gateway"),
    ("cloudflare-workers-ai", "Cloudflare Workers AI"),
    ("deepseek", "DeepSeek"),
    ("fireworks", "Fireworks"),
    ("google", "Google Gemini"),
    ("google-vertex", "Google Vertex AI"),
    ("groq", "Groq"),
    ("huggingface", "Hugging Face"),
    ("kimi-coding", "Kimi For Coding"),
    ("mistral", "Mistral"),
    ("minimax", "MiniMax"),
    ("minimax-cn", "MiniMax (China)"),
    ("moonshotai", "Moonshot AI"),
    ("moonshotai-cn", "Moonshot AI (China)"),
    ("nvidia", "NVIDIA NIM"),
    ("opencode", "OpenCode Zen"),
    ("opencode-go", "OpenCode Go"),
    ("openai", "OpenAI"),
    ("openrouter", "OpenRouter"),
    ("together", "Together AI"),
    ("vercel-ai-gateway", "Vercel AI Gateway"),
    ("xai", "xAI"),
    ("zai", "ZAI"),
    ("zai-coding-cn", "ZAI Coding Plan (China)"),
    ("xiaomi", "Xiaomi MiMo"),
    ("xiaomi-token-plan-cn", "Xiaomi MiMo Token Plan (China)"),
    (
        "xiaomi-token-plan-ams",
        "Xiaomi MiMo Token Plan (Amsterdam)",
    ),
    (
        "xiaomi-token-plan-sgp",
        "Xiaomi MiMo Token Plan (Singapore)",
    ),
];

/// Look up the human-readable display name for a provider.
/// Falls back to the raw provider ID if not found.
#[allow(dead_code)]
pub fn provider_display_name(provider_id: &str) -> &str {
    BUILT_IN_PROVIDER_DISPLAY_NAMES
        .iter()
        .find(|(id, _)| *id == provider_id)
        .map(|(_, name)| *name)
        .unwrap_or(provider_id)
}

// ── Provider-specific hosts ────────────────────────────────────────

#[allow(dead_code)]
const OPENROUTER_HOST: &str = "openrouter.ai";
#[allow(dead_code)]
const NVIDIA_NIM_HOST: &str = "integrate.api.nvidia.com";
#[allow(dead_code)]
const CLOUDFLARE_API_HOST: &str = "api.cloudflare.com";
#[allow(dead_code)]
const CLOUDFLARE_AI_GATEWAY_HOST: &str = "gateway.ai.cloudflare.com";
#[allow(dead_code)]
const OPENCODE_HOST: &str = "opencode.ai";
#[allow(dead_code)]
const VERCEL_GATEWAY_HOST: &str = "ai-gateway.vercel.sh";

#[allow(dead_code)]
fn matches_host(base_url: &str, expected_host: &str) -> bool {
    // Simple host extraction: find "://" then take up to "/" or ":"
    let after_scheme = match base_url.find("://") {
        Some(pos) => &base_url[pos + 3..],
        None => return base_url == expected_host,
    };
    let host = after_scheme
        .split(&['/', ':', '?'][..])
        .next()
        .unwrap_or(after_scheme);
    host == expected_host
}

// ── Attribution Headers ───────────────────────────────────────────

/// Get default attribution headers for a provider based on the model's provider type and base URL.
#[allow(dead_code)]
fn get_default_attribution_headers(
    provider: &str,
    base_url: &str,
) -> Option<HashMap<String, String>> {
    let mut headers = HashMap::new();

    match provider {
        _ if base_url.contains(OPENROUTER_HOST) => {
            headers.insert("HTTP-Referer".into(), "https://pi.dev".into());
            headers.insert("X-OpenRouter-Title".into(), "pi".into());
            headers.insert("X-OpenRouter-Categories".into(), "cli-agent".into());
        }
        _ if matches_host(base_url, NVIDIA_NIM_HOST) => {
            headers.insert("X-BILLING-INVOKE-ORIGIN".into(), "Pi".into());
        }
        "cloudflare-workers-ai" | "cloudflare-ai-gateway"
            if matches_host(base_url, CLOUDFLARE_API_HOST)
                || matches_host(base_url, CLOUDFLARE_AI_GATEWAY_HOST) =>
        {
            headers.insert("User-Agent".into(), "pi-coding-agent".into());
        }
        _ if matches_host(base_url, VERCEL_GATEWAY_HOST) => {
            headers.insert("http-referer".into(), "https://pi.dev".into());
            headers.insert("x-title".into(), "pi".into());
        }
        _ => return None,
    }

    Some(headers)
}

/// Get session tracking headers for OpenCode providers.
#[allow(dead_code)]
fn get_session_headers(
    provider: &str,
    base_url: &str,
    session_id: Option<&str>,
) -> Option<HashMap<String, String>> {
    let session_id = session_id?;

    let is_opencode =
        matches!(provider, "opencode" | "opencode-go") || matches_host(base_url, OPENCODE_HOST);

    if is_opencode {
        let mut headers = HashMap::new();
        headers.insert("x-opencode-session".into(), session_id.to_string());
        headers.insert("x-opencode-client".into(), "pi".into());
        Some(headers)
    } else {
        None
    }
}

/// Merge all provider attribution headers into a single map.
///
/// Combines session headers, default attribution headers, and any additional
/// header sources (e.g., custom headers from `ProviderConfig`).
#[allow(dead_code)]
pub fn merge_provider_attribution_headers(
    provider: &str,
    base_url: &str,
    session_id: Option<&str>,
    additional_headers: Option<&HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    let mut merged = HashMap::new();

    // Session headers (OpenCode)
    if let Some(session_headers) = get_session_headers(provider, base_url, session_id) {
        merged.extend(session_headers);
    }

    // Default attribution headers
    if let Some(attribution_headers) = get_default_attribution_headers(provider, base_url) {
        merged.extend(attribution_headers);
    }

    // Additional headers (e.g., custom headers from ProviderConfig)
    if let Some(extra) = additional_headers {
        merged.extend(extra.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    if merged.is_empty() {
        None
    } else {
        Some(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_display_name_found() {
        assert_eq!(provider_display_name("openrouter"), "OpenRouter");
        assert_eq!(provider_display_name("anthropic"), "Anthropic");
        assert_eq!(provider_display_name("openai"), "OpenAI");
    }

    #[test]
    fn test_provider_display_name_fallback() {
        assert_eq!(
            provider_display_name("unknown-provider"),
            "unknown-provider"
        );
    }

    #[test]
    fn test_openrouter_attribution() {
        let headers = merge_provider_attribution_headers(
            "openrouter",
            "https://openrouter.ai/api",
            None,
            None,
        );
        assert!(headers.is_some());
        let h = headers.unwrap();
        assert_eq!(h.get("HTTP-Referer").unwrap(), "https://pi.dev");
        assert_eq!(h.get("X-OpenRouter-Title").unwrap(), "pi");
        assert_eq!(h.get("X-OpenRouter-Categories").unwrap(), "cli-agent");
    }

    #[test]
    fn test_openrouter_by_base_url() {
        let headers =
            merge_provider_attribution_headers("custom", "https://openrouter.ai/v1", None, None);
        assert!(headers.is_some());
        let h = headers.unwrap();
        assert_eq!(h.get("HTTP-Referer").unwrap(), "https://pi.dev");
    }

    #[test]
    fn test_nvidia_attribution() {
        let headers = merge_provider_attribution_headers(
            "nvidia",
            "https://integrate.api.nvidia.com/v1",
            None,
            None,
        );
        assert!(headers.is_some());
        let h = headers.unwrap();
        assert_eq!(h.get("X-BILLING-INVOKE-ORIGIN").unwrap(), "Pi");
    }

    #[test]
    fn test_cloudflare_attribution() {
        let headers = merge_provider_attribution_headers(
            "cloudflare-workers-ai",
            "https://api.cloudflare.com/client/v4",
            None,
            None,
        );
        assert!(headers.is_some());
        let h = headers.unwrap();
        assert_eq!(h.get("User-Agent").unwrap(), "pi-coding-agent");
    }

    #[test]
    fn test_vercel_gateway_attribution() {
        let headers = merge_provider_attribution_headers(
            "vercel-ai-gateway",
            "https://ai-gateway.vercel.sh/v1",
            None,
            None,
        );
        assert!(headers.is_some());
        let h = headers.unwrap();
        assert_eq!(h.get("http-referer").unwrap(), "https://pi.dev");
        assert_eq!(h.get("x-title").unwrap(), "pi");
    }

    #[test]
    fn test_opencode_session_headers() {
        let headers = merge_provider_attribution_headers(
            "opencode",
            "https://opencode.ai/api",
            Some("session-123"),
            None,
        );
        assert!(headers.is_some());
        let h = headers.unwrap();
        assert_eq!(h.get("x-opencode-session").unwrap(), "session-123");
        assert_eq!(h.get("x-opencode-client").unwrap(), "pi");
    }

    #[test]
    fn test_opencode_without_session() {
        let headers =
            merge_provider_attribution_headers("opencode", "https://opencode.ai/api", None, None);
        assert!(headers.is_none());
    }

    #[test]
    fn test_unknown_provider_no_headers() {
        let headers = merge_provider_attribution_headers(
            "some-unknown-provider",
            "https://unknown.example.com",
            None,
            None,
        );
        assert!(headers.is_none());
    }

    #[test]
    fn test_additional_headers_merged() {
        let mut extra = HashMap::new();
        extra.insert("X-Custom".into(), "custom-value".into());

        let headers = merge_provider_attribution_headers(
            "openrouter",
            "https://openrouter.ai/api",
            None,
            Some(&extra),
        );
        assert!(headers.is_some());
        let h = headers.unwrap();
        assert_eq!(h.get("HTTP-Referer").unwrap(), "https://pi.dev");
        assert_eq!(h.get("X-Custom").unwrap(), "custom-value");
    }

    #[test]
    fn test_nvidia_by_host_match() {
        let headers = merge_provider_attribution_headers(
            "custom-openai",
            "https://integrate.api.nvidia.com/v1",
            None,
            None,
        );
        assert!(headers.is_some());
        let h = headers.unwrap();
        assert_eq!(h.get("X-BILLING-INVOKE-ORIGIN").unwrap(), "Pi");
    }
}
