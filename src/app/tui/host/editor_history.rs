//! Editor ↑/↓ send-history seed helpers (c1560 / ath12 split).

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::protocol::session::SessionEntry;

use super::HostSession;

impl<T: Terminal> HostSession<T> {
    /// Configure how many prior sessions seed ↑/↓ history on new session (c1560).
    pub fn set_editor_history_seed_sessions(&mut self, n: u32) {
        self.editor_history_seed_sessions = n;
    }

    /// Replace editor send history from already-loaded entries (resume/switch).
    pub fn seed_editor_history_from_entries(&mut self, entries: &[SessionEntry]) {
        let texts = super::super::editor_history_seed::user_prompt_texts_from_entries(entries);
        self.seed_editor_history_from_texts(texts);
    }

    /// Replace editor send history from prompt texts (background seed apply).
    pub fn seed_editor_history_from_texts(&mut self, texts: Vec<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().replace_editor_send_history(texts);
        }
    }

    /// Spawn ↑/↓ history seed when the driver exposes a cloneable store.
    ///
    /// Returns `None` when `n == 0` or the driver has no store (caller may await
    /// [`Self::seed_editor_history_for_new_session`] instead).
    pub fn kick_editor_history_seed(
        &self,
        driver: &dyn XyDriver,
    ) -> Option<tokio::task::JoinHandle<Vec<String>>> {
        let n = self.editor_history_seed_sessions;
        if n == 0 {
            return None;
        }
        let store = driver.session_store()?;
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| self.layout_cwd.clone());
        let current_id = driver.session_id();
        Some(super::super::editor_history_seed::spawn_new_session_seed(
            store, cwd, current_id, n,
        ))
    }

    /// Kick ↑/↓ history seed without awaiting (startup + `/session-new`).
    ///
    /// Returns `false` when the driver has no cloneable store — caller should
    /// fall back to [`Self::seed_editor_history_for_new_session`].
    /// Host loop takes the job via [`Self::take_editor_history_seed_job`].
    pub fn kick_editor_history_seed_async(&mut self, driver: &dyn XyDriver) -> bool {
        let Some(handle) = self.kick_editor_history_seed(driver) else {
            return false;
        };
        self.editor_history_seed_job = Some((std::time::Instant::now(), handle));
        true
    }

    /// Take a pending background seed job for the host `select!` arm.
    pub fn take_editor_history_seed_job(
        &mut self,
    ) -> Option<(std::time::Instant, tokio::task::JoinHandle<Vec<String>>)> {
        self.editor_history_seed_job.take()
    }

    /// Seed ↑/↓ history from prior same-cwd sessions (pure new session; blocking).
    pub async fn seed_editor_history_for_new_session(&mut self, driver: &dyn XyDriver) {
        let t = std::time::Instant::now();
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| self.layout_cwd.clone());
        let current_id = driver.session_id();
        let texts = super::super::editor_history_seed::collect_new_session_seed(
            driver,
            &cwd,
            current_id.as_deref(),
            self.editor_history_seed_sessions,
        )
        .await
        .unwrap_or_default();
        self.seed_editor_history_from_texts(texts);
        crate::app::core::lag::note("tui_seed_editor_history", t);
    }
}
