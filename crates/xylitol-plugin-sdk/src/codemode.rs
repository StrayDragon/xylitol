use std::collections::BTreeMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::ports::XyToolExposure;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CodemodeExposurePolicy {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
    pub read_only: bool,
    pub write_tools: Vec<String>,
}

impl CodemodeExposurePolicy {
    pub fn allows(&self, name: &str) -> bool {
        if self.deny.iter().any(|tool| tool == name) {
            return false;
        }
        if self.read_only && self.write_tools.iter().any(|tool| tool == name) {
            return false;
        }
        self.allow.is_empty() || self.allow.iter().any(|tool| tool == name)
    }
}

static EXPOSURE_POLICY: RwLock<Option<CodemodeExposurePolicy>> = RwLock::new(None);
static EXPOSURE_OVERRIDES: RwLock<BTreeMap<String, XyToolExposure>> = RwLock::new(BTreeMap::new());
static MAX_NESTED_CALLS: AtomicUsize = AtomicUsize::new(100);

pub fn set_codemode_exposure_policy(policy: Option<CodemodeExposurePolicy>) {
    *EXPOSURE_POLICY
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = policy;
}

pub fn codemode_exposure_policy() -> Option<CodemodeExposurePolicy> {
    EXPOSURE_POLICY
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

pub fn set_codemode_exposure_overrides(overrides: BTreeMap<String, XyToolExposure>) {
    *EXPOSURE_OVERRIDES
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = overrides;
}

pub fn codemode_exposure_overrides() -> BTreeMap<String, XyToolExposure> {
    EXPOSURE_OVERRIDES
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

pub fn set_codemode_max_nested_calls(max: usize) {
    MAX_NESTED_CALLS.store(max, Ordering::Relaxed);
}

pub fn codemode_max_nested_calls() -> usize {
    MAX_NESTED_CALLS.load(Ordering::Relaxed)
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CodemodeRuntime {
    #[default]
    Wasmtime,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CodemodeMode {
    #[default]
    On,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default, rename_all = "snake_case")]
pub struct CodemodeConfig {
    pub enabled: bool,
    pub runtime: CodemodeRuntime,
    pub mode: CodemodeMode,
    pub timeout_ms: u64,
    pub max_output_tokens: usize,
    pub max_result_bytes: usize,
    pub max_nested_calls: usize,
    pub fuel: u64,
    pub max_memory_bytes: usize,
    pub module_cache: bool,
    pub inline_budget: usize,
    pub exposure: BTreeMap<String, XyToolExposure>,
    #[schemars(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_exposure: Option<CodemodeExposurePolicy>,
}

impl Default for CodemodeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            runtime: CodemodeRuntime::Wasmtime,
            mode: CodemodeMode::On,
            timeout_ms: 120_000,
            max_output_tokens: 10_000,
            max_result_bytes: 100 * 1024,
            max_nested_calls: 100,
            fuel: 1_000_000_000,
            max_memory_bytes: 64 * 1024 * 1024,
            module_cache: true,
            inline_budget: 3_000,
            exposure: BTreeMap::new(),
            tool_exposure: None,
        }
    }
}

impl CodemodeConfig {
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}

pub fn run_code_selected(tools: &[String]) -> bool {
    tools.iter().any(|entry| {
        let entry = entry.trim();
        !entry.starts_with('-') && entry.trim_start_matches('+') == "run_code"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposure_policy_intersects_allow_deny_read_only() {
        let policy = CodemodeExposurePolicy {
            allow: vec!["read".into(), "bash".into()],
            deny: vec!["bash".into()],
            read_only: true,
            write_tools: vec!["write".into()],
        };
        assert!(policy.allows("read"));
        assert!(!policy.allows("bash"));
        assert!(!policy.allows("write"));
        assert!(!policy.allows("edit"));
    }
}
