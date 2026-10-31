pub mod defaults;

pub use xylitol_plugin_sdk::Compat;
pub use xylitol_plugin_sdk::model::XyPromptCacheRetention;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtraPolicy {
    pub prompt_cache_usage: bool,
    pub prompt_cache_key: bool,
    pub prompt_cache_retention: Option<XyPromptCacheRetention>,
    pub previous_response_id: bool,
}

impl Default for ExtraPolicy {
    fn default() -> Self {
        Self {
            prompt_cache_usage: defaults::PROMPT_CACHE_USAGE,
            prompt_cache_key: defaults::PROMPT_CACHE_KEY,
            prompt_cache_retention: None,
            previous_response_id: defaults::PREVIOUS_RESPONSE_ID,
        }
    }
}

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
    pub fn for_compat(compat: Compat) -> Self {
        match compat {
            Compat::Generic => Self::default(),
            Compat::Deepseek => Self {
                compat: Compat::Deepseek,
                extra_policy: ExtraPolicy {
                    prompt_cache_usage: true,
                    prompt_cache_key: false,
                    prompt_cache_retention: None,
                    previous_response_id: false,
                },
            },
        }
    }

    pub fn expects_prompt_cache_usage(self) -> bool {
        self.extra_policy.prompt_cache_usage
    }

    pub fn allows_prompt_cache_key(self) -> bool {
        self.extra_policy.prompt_cache_key
    }

    pub fn prompt_cache_retention(self) -> Option<XyPromptCacheRetention> {
        self.extra_policy.prompt_cache_retention
    }

    pub fn allows_previous_response_id(self) -> bool {
        self.extra_policy.previous_response_id
    }

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
        assert_eq!(p.prompt_cache_retention(), None);
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
                prompt_cache_retention: Some(XyPromptCacheRetention::TwentyFourHours),
                previous_response_id: true,
            },
        };
        assert!(p.expects_prompt_cache_usage());
        assert!(!p.allows_prompt_cache_key());
        assert!(p.allows_previous_response_id());
        assert!(p.allows_reasoning_encrypted_include());
        assert_eq!(
            p.prompt_cache_retention(),
            Some(XyPromptCacheRetention::TwentyFourHours)
        );
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
        assert_eq!(p.prompt_cache_retention(), None);
    }

    #[test]
    fn extra_policy_has_no_agent_capability_fields() {
        let ep = ExtraPolicy::default();
        let _ = (
            ep.prompt_cache_usage,
            ep.prompt_cache_key,
            ep.prompt_cache_retention,
            ep.previous_response_id,
        );
    }
}
