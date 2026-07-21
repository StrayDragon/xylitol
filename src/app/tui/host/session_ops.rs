//! Session tree / models / resume mount + apply helpers (c1170 / ath12).

use crate::app::core::driver::{ModelInfo, XyDriver};
use crate::protocol::session::{SessionEntry, SessionTreeTravel};
use xylitol_tui::Terminal;
use xylitol_tui::TreeNode;
use xylitol_tui::components::select_list::SelectItem;

use super::super::bridge::session_tree::rebuild_scrollback_from_travel;
use super::HostSession;

impl<T: Terminal> HostSession<T> {
    pub fn take_pending_session_tree_open(&mut self) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        root.borrow_mut().take_pending_tree_open()
    }

    /// Queue a MessageHistory tree open (c700/c1005 `/session-tree`).
    pub fn request_session_tree_open(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().request_tree_open();
        }
    }

    /// Queue fork at `entry_id` (c700/c1005 `/session-fork` / tree Shift+F).
    pub fn request_session_tree_fork(&mut self, entry_id: impl Into<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().request_tree_fork(entry_id.into());
        }
    }

    pub fn take_pending_session_tree_travel(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_tree_travel()
    }

    pub fn take_pending_session_tree_fork(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_tree_fork()
    }

    pub fn take_pending_session_tree_label(&mut self) -> Option<(String, Option<String>)> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_tree_label()
    }

    pub fn apply_session_tree_label(&mut self, id: &str, label: Option<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().apply_tree_label(id, label);
        }
    }

    pub fn mount_session_tree(&mut self, roots: Vec<TreeNode>, active_id: Option<String>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut()
            .mount_session_tree(roots, active_id.as_deref());
        self.sync_ui_root_from_model();
    }

    pub fn take_pending_model_select(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_model_select()
    }

    pub fn take_pending_theme_select(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_theme_select()
    }

    pub fn take_pending_import_decision(
        &mut self,
    ) -> Option<super::super::layout::ImportConfirmDecision> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_import_decision()
    }

    pub fn mount_import_confirm(&mut self, path: &str) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().mount_import_confirm(path);
        self.sync_ui_root_from_model();
    }

    pub fn close_import_confirm(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().close_import_confirm();
            self.sync_ui_root_from_model();
        }
    }

    pub fn mount_session_resume_picker(
        &mut self,
        entries: Vec<crate::app::core::driver::SessionListEntry>,
        current_id: Option<String>,
    ) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        {
            let mut root = root.borrow_mut();
            root.mount_session_resume_picker(entries, current_id.as_deref());
        }
        self.sync_ui_root_from_model();
    }

    pub fn mount_session_resume_loading(&mut self, loaded: usize, total: usize) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut()
                .mount_session_resume_loading(loaded, total);
            self.sync_ui_root_from_model();
        }
    }

    pub fn take_pending_session_resume_select(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_session_resume_select()
    }

    pub fn take_pending_session_resume_rename(&mut self) -> Option<(String, String)> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_session_resume_rename()
    }

    pub fn take_pending_session_resume_delete(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        root.borrow_mut().take_pending_session_resume_delete()
    }

    pub fn session_resume_apply_rename(&mut self, id: &str, name: &str) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().session_resume.apply_rename(id, name);
            self.sync_ui_root_from_model();
        }
    }

    pub fn session_resume_remove_entry(&mut self, id: &str) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().session_resume.remove_entry(id);
            self.sync_ui_root_from_model();
        }
    }

    pub fn session_resume_set_status(&mut self, msg: impl Into<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().session_resume.set_status(msg);
            self.sync_ui_root_from_model();
        }
    }

    pub fn close_session_resume_slot(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().close_session_resume();
            self.sync_ui_root_from_model();
        }
    }

    pub fn mount_models_picker(&mut self, models: Vec<ModelInfo>, current_id: Option<String>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        let catalog: Vec<(String, String)> = models
            .iter()
            .map(|m| {
                let desc = if m.display_name.is_empty() {
                    String::new()
                } else {
                    m.display_name.clone()
                };
                (m.id.clone(), desc)
            })
            .collect();
        let items = models
            .iter()
            .map(|m| model_info_to_select_item(m, &current_id))
            .collect();
        {
            let mut root = root.borrow_mut();
            root.set_model_arg_catalog(catalog);
            root.mount_models_picker(items);
        }
        self.sync_ui_root_from_model();
    }

    /// Seed `/model <id>` completion catalog without opening the picker (c999).
    pub fn set_model_arg_catalog_from_models(&mut self, models: &[ModelInfo]) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        let catalog = models
            .iter()
            .map(|m| {
                let desc = if m.display_name.is_empty() {
                    String::new()
                } else {
                    m.display_name.clone()
                };
                (m.id.clone(), desc)
            })
            .collect();
        root.borrow_mut().set_model_arg_catalog(catalog);
    }

    /// Seed `$skill` completion catalog (c1130).
    pub fn set_dollar_skill_catalog(&mut self, catalog: Vec<(String, String)>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().set_dollar_skill_catalog(catalog);
    }

    /// Refresh loaded-resources header from XyDriver (c1135). Startup + `/reload`.
    pub async fn refresh_loaded_resources(&mut self, driver: &dyn XyDriver) {
        let snap = driver.loaded_resources_snapshot().await;
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().set_loaded_resources(snap);
    }

    pub fn close_models_slot(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().close_slot();
            self.sync_ui_root_from_model();
        }
    }

    pub fn mount_themes_picker(&mut self) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        let current = self.theme_preference.clone();
        root.borrow_mut().mount_themes_picker(current.as_deref());
        self.sync_ui_root_from_model();
    }

    pub fn close_themes_slot(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().close_slot();
            self.sync_ui_root_from_model();
        }
    }

    pub fn apply_session_tree_travel(
        &mut self,
        travel: SessionTreeTravel,
        entries: Vec<SessionEntry>,
    ) {
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.close_session_tree();
            if let Some(text) = travel.editor_text {
                root.set_editor_text(text);
            } else {
                root.set_editor_text(String::new());
            }
        }
        self.sync_ui_root_from_model();
    }

    /// After XyDriver fork+switch: rebuild transcript from child entries and optional prefill.
    pub fn apply_session_tree_fork(
        &mut self,
        child_id: &str,
        entries: Vec<SessionEntry>,
        editor_prefill: Option<String>,
    ) {
        let leaf_id = entries
            .iter()
            .rev()
            .find_map(|e| e.entry_id().map(str::to_string));
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: child_id.to_string(),
            leaf_id,
            editor_text: editor_prefill.clone(),
        };
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.close_session_tree();
            root.set_editor_text(editor_prefill.unwrap_or_default());
        }
        self.sync_ui_root_from_model();
        // Single trailing note (same family as clone/resume); do not also rewrite the
        // history @ banner — that duplicated the fork notice at top and bottom.
        self.push_system_note(format!("forked → session {child_id}"));
    }

    /// After import + switch: rebuild transcript from imported entries (c1010).
    pub fn apply_import_session(&mut self, session_id: &str, entries: Vec<SessionEntry>) {
        self.apply_switched_session(
            session_id,
            entries,
            format!("imported → session {session_id}"),
            |root| {
                root.close_import_confirm();
            },
        );
    }

    /// After `/session-resume` switch: rebuild transcript and clear overlays (c1015).
    pub fn apply_resume_session(&mut self, session_id: &str, entries: Vec<SessionEntry>) {
        self.apply_switched_session(
            session_id,
            entries,
            format!("switched → session {session_id}"),
            |root| {
                root.close_session_tree();
                root.close_session_resume();
            },
        );
    }

    /// After `/session-new`: empty (or near-empty) transcript (c1020).
    pub fn apply_new_session(&mut self, session_id: &str, entries: Vec<SessionEntry>) {
        self.apply_switched_session(
            session_id,
            entries,
            format!("new session → {session_id}"),
            |root| {
                root.close_session_tree();
                root.close_session_resume();
            },
        );
    }

    /// After `/session-clone` (fork At + switch) (c1020).
    pub fn apply_clone_session(&mut self, session_id: &str, entries: Vec<SessionEntry>) {
        self.apply_switched_session(
            session_id,
            entries,
            format!("cloned → session {session_id}"),
            |root| {
                root.close_session_tree();
            },
        );
    }

    fn apply_switched_session(
        &mut self,
        session_id: &str,
        entries: Vec<SessionEntry>,
        note: String,
        close_overlays: impl FnOnce(&mut super::super::layout::UiRoot),
    ) {
        let leaf_id = entries
            .iter()
            .rev()
            .find_map(|e| e.entry_id().map(str::to_string));
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: session_id.to_string(),
            leaf_id,
            editor_text: None,
        };
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            close_overlays(&mut root);
            root.set_editor_text(String::new());
        }
        self.sync_ui_root_from_model();
        self.push_system_note(note);
    }

    /// Apply `/debug <scene>` load: rebuild transcript and optional footer model (c710).
    pub fn apply_debug_scene(&mut self, load: crate::app::core::driver::DebugSceneLoad) {
        let leaf_id = load
            .entries
            .iter()
            .rev()
            .find_map(|e| e.entry_id().map(str::to_string));
        let travel = SessionTreeTravel {
            kind: crate::protocol::session::SessionTreeKind::MessageHistory,
            selected_id: leaf_id.clone().unwrap_or_else(|| load.session_id.clone()),
            leaf_id,
            editor_text: None,
        };
        rebuild_scrollback_from_travel(&mut self.ui_model, &load.entries, &travel);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.close_session_tree();
            root.set_editor_text(String::new());
        }
        if let Some(m) = load.model {
            let label = if m.display_name.is_empty() {
                m.id
            } else {
                m.display_name
            };
            self.set_footer_model(label);
        }
        self.sync_ui_root_from_model();
        self.push_system_note(load.note);
    }
}

fn model_info_to_select_item(m: &ModelInfo, current_id: &Option<String>) -> SelectItem {
    let label = if m.display_name.is_empty() {
        m.id.clone()
    } else {
        m.display_name.clone()
    };
    let marked = if current_id.as_deref() == Some(m.id.as_str()) {
        format!("{label} *")
    } else {
        label
    };
    SelectItem::new(m.id.clone(), marked)
}
