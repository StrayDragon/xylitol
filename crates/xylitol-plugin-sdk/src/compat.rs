use strum::{EnumString, IntoStaticStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, IntoStaticStr, EnumString)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Compat {
    #[default]
    Generic,
    Deepseek,
}

impl Compat {
    pub fn as_str(self) -> &'static str {
        self.into()
    }

    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Some(Self::Generic);
        }
        s.parse().ok()
    }

    pub fn allows_reasoning_encrypted_include(self) -> bool {
        matches!(self, Self::Generic)
    }
}

impl std::fmt::Display for Compat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
