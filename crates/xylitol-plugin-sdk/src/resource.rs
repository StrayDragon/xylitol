use std::path::PathBuf;

use crate::source_info::SourceInfo;

#[derive(Debug, Clone)]
pub struct AgentsFile {
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ResourceDiagnostic {
    pub level: String,
    pub message: String,
    pub path: Option<PathBuf>,
}

impl ResourceDiagnostic {
    pub fn error(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self {
            level: "error".into(),
            message: message.into(),
            path,
        }
    }

    pub fn warning(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self {
            level: "warning".into(),
            message: message.into(),
            path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub description: Option<String>,
    pub source_info: SourceInfo,
    pub disable_model_invocation: bool,
}

#[derive(Debug, Clone)]
pub struct ThemeInfo {
    pub name: String,
    pub source_info: SourceInfo,
}
