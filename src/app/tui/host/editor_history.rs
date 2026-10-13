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

    /// c2470：接入 client-local 存储通道（None = 不落地，labs / 测试缺省）。
    pub fn set_client_storage(
        &mut self,
        storage: Option<std::sync::Arc<crate::app::tui::storage::ClientStorage>>,
    ) {
        self.client_storage = storage;
    }

    /// c2470：接 watcher 上报通道（跨实例变更 key）。
    pub fn set_client_history_receiver(&mut self, rx: std::sync::mpsc::Receiver<String>) {
        self.client_history_rx = Some(rx);
    }

    /// c2470：发送落盘（best-effort；失败静默，最坏丢一条本地历史）。
    pub(crate) fn persist_client_history(&mut self, text: &str) {
        if let Some(storage) = &self.client_storage {
            storage.record_editor_prompt(text);
        }
    }

    /// c2470：通道历史并入 ↑/↓（去重置顶；无通道 / 空历史 no-op）。
    pub fn merge_client_history(&mut self) {
        let Some(storage) = &self.client_storage else {
            return;
        };
        let texts = storage.editor_history();
        if texts.is_empty() {
            return;
        }
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().merge_editor_history(texts);
        }
    }

    /// c2470：tick 轮询 watcher 变更；编辑器历史变更 → 重载合并。
    pub fn drain_client_history_updates(&mut self) {
        let has_updates = {
            let Some(rx) = &mut self.client_history_rx else {
                return;
            };
            let mut hit = false;
            while let Ok(key) = rx.try_recv() {
                if key == crate::app::tui::storage::KEY_EDITOR_HISTORY {
                    hit = true;
                }
            }
            hit
        };
        if has_updates {
            self.merge_client_history();
        }
    }

    pub fn set_activity_fold_settings(
        &mut self,
        settings: crate::app::tui::activity_fold::ActivityFoldSettings,
    ) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().set_activity_settings(settings);
        }
    }

    /// Replace editor send history from already-loaded entries (resume/switch).
    pub fn seed_editor_history_from_entries(&mut self, entries: &[SessionEntry]) {
        let texts = super::super::editor_history_seed::user_prompt_texts_from_entries(entries);
        self.seed_editor_history_from_texts(texts);
    }

    /// Replace editor send history from prompt texts (background seed apply).
    /// c2470：替换后并回 client-local 通道历史（切换 / 恢复 / 启动统一走此钩子）。
    pub fn seed_editor_history_from_texts(&mut self, texts: Vec<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().replace_editor_send_history(texts);
        }
        self.merge_client_history();
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
    pub async fn seed_editor_history_for_new_session(&mut self, driver: &mut dyn XyDriver) {
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
