//! Test helpers for config modules.
//!
//! These helpers exist to serialize tests that mutate process-wide environment variables. Rust
//! tests run in parallel by default, so per-module locks are not sufficient.

use std::sync::Mutex;

/// Serialize env-modifying tests to avoid cross-test races.
pub(crate) static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Guard to restore env vars on drop.
pub(crate) struct EnvGuard(Vec<(String, Option<String>)>);

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.0 {
            match v {
                Some(val) => unsafe { std::env::set_var(k, val) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }
}

pub(crate) fn save_env(keys: &[&str]) -> EnvGuard {
    EnvGuard(
        keys.iter()
            .map(|k| (k.to_string(), std::env::var(k).ok()))
            .collect(),
    )
}
