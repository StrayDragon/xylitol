//! Test helpers for config modules.
//!
//! These helpers exist to serialize tests that mutate process-wide environment variables. Rust
//! tests run in parallel by default, so per-module locks are not sufficient.
//!
//! # Safety: env-var mutation in tests
//!
//! All `unsafe` invocations in this module and in callers (paths.rs, loader.rs) manipulate
//! `std::env::set_var` / `std::env::remove_var`. These are `unsafe` in Rust because they mutate
//! process-global state observable by all threads — unsound in a multi-threaded program.
//!
//! We make each `unsafe` call sound by combining two safeguards:
//!
//! 1. **`ENV_LOCK`** — a global `Mutex<()>` acquired at the _top_ of every env-modifying test.
//!    This serializes all such tests globally (not just per-crate).
//!
//! 2. **`EnvGuard`** — records current values on construction and restores them on drop.
//!    This guarantees no env change leaks out of the test scope, even on panic.
//!
//! With these in place, no two threads can observe conflicting env state, and every
//! modification is reverted before the next test acquires the lock.

use std::sync::Mutex;

/// Serialize env-modifying tests to avoid cross-test races.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Guard to restore env vars on drop.
///
/// # Safety
///
/// The `unsafe` in `Drop::drop` is sound because:
/// - The lock held by the test guarantees exclusive access to the process env.
/// - `EnvGuard` only restores values that the test itself mutated during
///   the same lock acquisition window.
pub struct EnvGuard(Vec<(String, Option<String>)>);

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.0 {
            // SAFETY: ENV_LOCK is held by the calling test, guaranteeing
            // exclusive access to the process-global environment.
            match v {
                Some(val) => unsafe { std::env::set_var(k, val) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }
}

pub fn save_env(keys: &[&str]) -> EnvGuard {
    EnvGuard(
        keys.iter()
            .map(|k| (k.to_string(), std::env::var(k).ok()))
            .collect(),
    )
}
