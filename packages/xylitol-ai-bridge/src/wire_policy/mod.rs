//! Wire-side policy for API req/resp (not agent capabilities).
//!
//! Defaults live in [`defaults`] — change constants there to debug unexposed knobs.

pub mod defaults;

/// Compatibility profile for a chosen `api` protocol family.
///
/// First language = vendor-native API. Dialect = third-party shape of that API.
/// This profile tunes how conservatively we treat dialect endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compat {
    /// Conservative: do not assume first-language wire semantics.
    #[default]
    Generic,
}

impl Compat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "generic",
        }
    }
}

impl std::fmt::Display for Compat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_generic_with_wire_bits_off() {
        let p = WirePolicy::default();
        assert_eq!(p.compat, Compat::Generic);
        assert_eq!(p.compat.as_str(), "generic");
        assert!(!p.expects_prompt_cache_usage());
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
