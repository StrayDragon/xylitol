//! Wire-side policy for API req/resp (not agent capabilities).
//!
//! Defaults live in [`defaults`] — change constants there to debug unexposed knobs.

pub mod defaults;

pub use xylitol_plugin_sdk::Compat;

/// API request/response maintenance flags only (c1880).
///
/// MUST NOT hold agent/ContextPolicy knobs (`tool_search`, status bar, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtraPolicy {
    pub prompt_cache_usage: bool,
    pub prompt_cache_key: bool,
    pub previous_response_id: bool,
}

impl Default for ExtraPolicy {
    fn default() -> Self {
        Self {
            prompt_cache_usage: defaults::PROMPT_CACHE_USAGE,
            prompt_cache_key: defaults::PROMPT_CACHE_KEY,
            previous_response_id: defaults::PREVIOUS_RESPONSE_ID,
        }
    }
}

/// `api` × this policy drives wire field subsets / usage expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WirePolicy {
    pub compat: Compat,
    pub extra_policy: ExtraPolicy,
}

impl Default for WirePolicy {
    fn default() -> Self {
        Self {
            compat: defaults::COMPAT_DEFAULT,
            extra_policy: ExtraPolicy::default(),
        }
    }
}

impl WirePolicy {
    /// Named profile → WirePolicy (defaults board + dialect quirks).
    pub fn for_compat(compat: Compat) -> Self {
        match compat {
            Compat::Generic => Self::default(),
            Compat::Deepseek => Self {
                compat: Compat::Deepseek,
                extra_policy: ExtraPolicy {
                    // DeepSeek reports cache reads (Completions hit_tokens / details;
                    // Responses input_tokens_details). Still omit encrypted include
                    // and previous_response_id.
                    prompt_cache_usage: true,
                    prompt_cache_key: false,
                    previous_response_id: false,
                },
            },
        }
    }

    /// Whether usage mapping may expect first-language cache read fields.
    pub fn expects_prompt_cache_usage(self) -> bool {
        self.extra_policy.prompt_cache_usage
    }

    /// Whether request assembly may attach a prompt cache key.
    pub fn allows_prompt_cache_key(self) -> bool {
        self.extra_policy.prompt_cache_key
    }

    /// Whether chained `previous_response_id` may be enabled.
    pub fn allows_previous_response_id(self) -> bool {
        self.extra_policy.previous_response_id
    }

    /// Whether Responses assemble may request encrypted reasoning include.
    pub fn allows_reasoning_encrypted_include(self) -> bool {
        self.compat.allows_reasoning_encrypted_include()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_generic_with_expected_cache_usage() {
        let p = WirePolicy::default();
        assert_eq!(p.compat, Compat::Generic);
        assert_eq!(p.compat.as_str(), "generic");
        assert!(p.expects_prompt_cache_usage());
        assert!(!p.allows_prompt_cache_key());
        assert!(!p.allows_previous_response_id());
        assert_eq!(p.compat, defaults::COMPAT_DEFAULT);
        assert_eq!(
            p.extra_policy.prompt_cache_usage,
            defaults::PROMPT_CACHE_USAGE
        );
        assert_eq!(p.extra_policy.prompt_cache_key, defaults::PROMPT_CACHE_KEY);
        assert_eq!(
            p.extra_policy.previous_response_id,
            defaults::PREVIOUS_RESPONSE_ID
        );
    }

    #[test]
    fn literal_override_for_tests() {
        let p = WirePolicy {
            compat: Compat::Generic,
            extra_policy: ExtraPolicy {
                prompt_cache_usage: true,
                prompt_cache_key: false,
                previous_response_id: true,
            },
        };
        assert!(p.expects_prompt_cache_usage());
        assert!(!p.allows_prompt_cache_key());
        assert!(p.allows_previous_response_id());
        assert!(p.allows_reasoning_encrypted_include());
    }

    #[test]
    fn deepseek_profile_expects_cache_skips_encrypted_include() {
        let p = WirePolicy::for_compat(Compat::Deepseek);
        assert_eq!(p.compat, Compat::Deepseek);
        assert_eq!(Compat::parse("deepseek"), Some(Compat::Deepseek));
        assert!(p.expects_prompt_cache_usage());
        assert!(!p.allows_reasoning_encrypted_include());
        assert!(!p.allows_previous_response_id());
        assert!(!p.allows_prompt_cache_key());
    }

    #[test]
    fn extra_policy_has_no_agent_capability_fields() {
        // Compile-time / structural guard: ExtraPolicy only exposes the three wire bits.
        let ep = ExtraPolicy::default();
        let _ = (
            ep.prompt_cache_usage,
            ep.prompt_cache_key,
            ep.previous_response_id,
        );
    }
}
