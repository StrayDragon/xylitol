use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(default)]
pub struct ContextInjectConfig {
    pub global: Vec<InjectorConfig>,
    pub project: Vec<InjectorConfig>,
    pub user: Vec<InjectorConfig>,
}

impl ContextInjectConfig {
    pub fn validate(&self) -> Result<(), String> {
        for tier in [&self.global, &self.project, &self.user] {
            let mut ids = std::collections::HashSet::new();
            for injector in tier {
                if injector.id.trim().is_empty() {
                    return Err("context_inject injector id must not be empty".into());
                }
                if !ids.insert(injector.id.clone()) {
                    return Err(format!(
                        "duplicate context_inject injector id: {}",
                        injector.id
                    ));
                }
                if injector.command.trim().is_empty() {
                    return Err(format!(
                        "context_inject injector {} command must not be empty",
                        injector.id
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn merged(&self) -> Vec<InjectorConfig> {
        let mut merged: Vec<InjectorConfig> = Vec::new();
        for tier in [&self.global, &self.project, &self.user] {
            for injector in tier {
                match merged
                    .iter()
                    .position(|existing| existing.id == injector.id)
                {
                    Some(index) => merged[index] = injector.clone(),
                    None => merged.push(injector.clone()),
                }
            }
        }
        merged
    }

    pub fn is_active(&self) -> bool {
        !self.global.is_empty() || !self.project.is_empty() || !self.user.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InjectorConfig {
    pub id: String,
    pub point: InjectorPoint,
    pub command: String,
    #[serde(default)]
    pub placement: Placement,
    #[serde(default)]
    pub dedup: DedupMode,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InjectorPoint {
    SessionStart,
    SessionShutdown,
    TurnStart,
    TurnEnd,
    TurnPre,
    StepStart,
    StepEnd,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    #[default]
    HistoryRow,
    Ephemeral,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DedupMode {
    #[default]
    None,
    LatestWins,
    SuppressIfUnchanged,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_ids_are_rejected() {
        let config = ContextInjectConfig {
            global: vec![
                InjectorConfig {
                    id: "a".into(),
                    point: InjectorPoint::TurnPre,
                    command: "true".into(),
                    placement: Placement::Ephemeral,
                    dedup: DedupMode::None,
                    timeout_secs: None,
                    group: None,
                },
                InjectorConfig {
                    id: "a".into(),
                    point: InjectorPoint::StepStart,
                    command: "true".into(),
                    placement: Placement::Ephemeral,
                    dedup: DedupMode::None,
                    timeout_secs: None,
                    group: None,
                },
            ],
            project: Vec::new(),
            user: Vec::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn empty_command_is_rejected() {
        let config = ContextInjectConfig {
            global: vec![InjectorConfig {
                id: "a".into(),
                point: InjectorPoint::TurnPre,
                command: "  ".into(),
                placement: Placement::Ephemeral,
                dedup: DedupMode::None,
                timeout_secs: None,
                group: None,
            }],
            project: Vec::new(),
            user: Vec::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn valid_config_passes() {
        let config = ContextInjectConfig {
            user: vec![InjectorConfig {
                id: "git-status".into(),
                point: InjectorPoint::TurnPre,
                command: "git status --short".into(),
                placement: Placement::Ephemeral,
                dedup: DedupMode::SuppressIfUnchanged,
                timeout_secs: Some(2),
                group: None,
            }],
            global: Vec::new(),
            project: Vec::new(),
        };
        assert!(config.validate().is_ok());
        assert!(config.is_active());
    }
}
