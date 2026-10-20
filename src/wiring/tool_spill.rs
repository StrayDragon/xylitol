//! Process-wide tool spill directory (c2080).
//!
//! Composition roots / surfaces call [`install`] so tool output spill files
//! write beside the session instead of a global system tmp. Surfaces MUST NOT
//! import config / provider modules for this — use this seam.

use std::path::PathBuf;
use std::sync::Mutex;

#[allow(dead_code)] // composition-root seam; install happens when Driver materializes
static SPILL_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Install (or clear) the process spill root for tool hard-truncation.
#[allow(dead_code)] // composition-root seam; install happens when Driver materializes
pub fn install(dir: Option<PathBuf>) {
    *SPILL_DIR.lock().expect("spill dir lock") = dir;
}

/// Current process spill root (tests / diagnostics).
#[allow(dead_code)] // composition-root seam; install happens when Driver materializes
pub fn current() -> Option<PathBuf> {
    SPILL_DIR.lock().expect("spill dir lock").clone()
}
