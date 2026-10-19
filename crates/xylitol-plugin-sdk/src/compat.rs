//! Named wire/thinking dialect profile (`generic` | `deepseek`).

use strum::{EnumString, IntoStaticStr};

/// Compatibility profile for a chosen `api` protocol family.
///
/// First language = vendor-native API. Dialect = third-party shape of that API.
/// Named profiles map YAML `models.*.compat` → provider wire policy (c1940).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, IntoStaticStr, EnumString)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Compat {
    /// Conservative: do not assume first-language wire semantics.
    #[default]
    Generic,
    /// DeepSeek-shaped dialect (Responses: no encrypted include; Completions: `thinking.type`;
    /// Anthropic Messages: `thinking.type` without `budget_tokens`).
    Deepseek,
}

impl Compat {
    pub fn as_str(self) -> &'static str {
        self.into()
    }

    /// Parse a YAML / config `compat` string (`generic` | `deepseek`). Empty → Generic.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Some(Self::Generic);
        }
        s.parse().ok()
    }

    /// Whether Responses assemble may request `include: reasoning.encrypted_content`.
    pub fn allows_reasoning_encrypted_include(self) -> bool {
        matches!(self, Self::Generic)
    }
}

impl std::fmt::Display for Compat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
