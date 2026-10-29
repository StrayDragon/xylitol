use crate::bridge::ModeStripState;
use crate::debug_fixtures::FixedZoneOp;
use crate::driver::{ModelInfo, XyDriver};
use crate::layout::{WidthTier, width_tier};
use xylitol_plugin_sdk::session::{
    PlanModeKind, SessionEntry, SessionTreeTravel, latest_plan_mode,
};
use xylitol_tui::{ChoiceMode, ChoiceOption, ChoiceQuestion, Terminal, TreeNode};

use super::super::bridge::session_tree::{rebuild_scrollback_from_travel, travel_history_note};
use super::HostSession;

impl<T: Terminal> HostSession<T> {
    pub fn sync_session_sidebar_reserve(&mut self) {
        let cols = Terminal::columns(&self.tui.terminal) as usize;
        let n = self
            .ui_root
            .as_ref()
            .map(|r| r.borrow().session_sidebar_reserved_cols(cols))
            .unwrap_or(0);
        self.tui.set_extra_reserved_cols(n as u16);
    }

    pub fn session_sidebar_available(&self) -> bool {
        width_tier(Terminal::columns(&self.tui.terminal) as usize) >= WidthTier::Standard
    }

    pub fn take_pending_session_sidebar(&mut self) -> bool {
        let root = self.ui_root.as_ref().expect("product ui").clone();
        root.borrow_mut().take_pending_session_sidebar()
    }

    pub fn show_session_sidebar(&mut self, session_id: &str) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().show_session_sidebar(session_id);
        }
    }

    pub fn take_pending_session_popup(&mut self) -> bool {
        let root = self.ui_root.as_ref().expect("product ui").clone();
        root.borrow_mut().take_pending_session_popup()
    }

    pub fn show_session_popup(&mut self, session_id: &str) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().show_session_popup(session_id);
        }
    }

    pub fn take_pending_session_tree_open(&mut self) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        root.borrow_mut().take_pending_tree_open()
    }

    pub fn request_session_tree_open(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().request_tree_open();
        }
    }

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

    pub fn take_pending_model_select(&mut self) -> Option<crate::layout::PendingModelChoice> {
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

    pub fn mount_ask_choice(
        &mut self,
        questions: Vec<xylitol_tui::ChoiceQuestion>,
        reply: tokio::sync::oneshot::Sender<Result<String, xylitol_plugin_sdk::error::XyToolError>>,
    ) {
        let Some(root) = self.ui_root.as_ref() else {
            let _ = reply.send(Err(
                xylitol_plugin_sdk::error::XyToolError::ExecutionFailed(anyhow::anyhow!(
                    "TUI root unavailable for ask"
                )),
            ));
            return;
        };
        root.borrow_mut().mount_ask_choice(questions, reply);
        self.sync_ui_root_from_model();
    }

    pub fn arm_debug_live_ask_close(&mut self) {
        self.debug_live_ask_close = true;
    }

    pub fn complete_ask_if_ready(&mut self) -> Option<String> {
        let root = self.ui_root.as_ref()?;
        let json = root.borrow_mut().complete_ask_if_ready()?;
        self.sync_ui_root_from_model();
        Some(json)
    }

    pub fn poll_ask_host(&mut self) {
        if let Some(json) = self.complete_ask_if_ready() {
            if self.debug_live_ask_close {
                self.debug_live_ask_close = false;
                self.apply_debug_live_ask_close(&json);
            }
            let _ = self.render_now();
        }
        let Some(gw) = self.ask_gateway.clone() else {
            return;
        };
        let Some(pending) = gw.take_pending() else {
            return;
        };
        let questions = crate::ask_host::ask_questions_to_choice(pending.questions);
        self.mount_ask_choice(questions, pending.reply);
        let _ = self.render_now();
    }

    fn apply_debug_live_ask_close(&mut self, json: &str) {
        for event in crate::activity_fold::live_ask_close_events(json) {
            self.apply_xy_unfenced(Box::new(event));
        }
    }

    pub fn mount_session_resume_picker(
        &mut self,
        entries: Vec<crate::driver::SessionListEntry>,
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
            root.borrow_mut().session_resume_apply_rename(id, name);
            self.sync_ui_root_from_model();
        }
    }

    pub fn session_resume_remove_entry(&mut self, id: &str) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().session_resume_remove_entry(id);
            self.sync_ui_root_from_model();
        }
    }

    pub fn session_resume_set_status(&mut self, msg: impl Into<String>) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().session_resume_set_status(msg);
            self.sync_ui_root_from_model();
        }
    }

    pub fn close_session_resume_slot(&mut self) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut().close_session_resume();
            self.sync_ui_root_from_model();
        }
    }

    pub fn mount_models_picker(
        &mut self,
        models: Vec<ModelInfo>,
        current_id: Option<String>,
        current_thinking: String,
    ) {
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
        let rows = models
            .iter()
            .map(|m| {
                crate::layout::ModelPickerRow::from_info(
                    m,
                    current_id.as_deref(),
                    current_thinking.clone(),
                )
            })
            .collect();
        {
            let mut root = root.borrow_mut();
            root.set_model_arg_catalog(catalog);
            root.mount_models_picker(rows);
        }
        self.sync_ui_root_from_model();
    }

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

    pub fn set_dollar_skill_catalog(&mut self, catalog: Vec<(String, String)>) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().set_dollar_skill_catalog(catalog);
    }

    pub async fn refresh_loaded_resources(&mut self, driver: &dyn XyDriver) {
        let snap = driver.loaded_resources_snapshot().await;
        self.refresh_loaded_resources_from_snap(snap);
    }

    pub fn refresh_loaded_resources_from_snap(
        &mut self,
        snap: crate::driver::LoadedResourcesSnapshot,
    ) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().set_loaded_resources(snap);
    }

    pub fn mcp_cache_usable_for_open(&self) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        root.borrow().mcp_cache_usable_for_open()
    }

    pub fn mount_mcp_from_cache(&mut self) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().mount_mcp_from_loaded_resources();
        self.sync_ui_root_from_model();
    }

    pub fn mount_mcp_panel(&mut self, snap: &crate::driver::LoadedResourcesSnapshot) {
        let Some(root) = self.ui_root.as_ref() else {
            return;
        };
        root.borrow_mut().mount_mcp_panel(snap);
        self.sync_ui_root_from_model();
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
        let note = travel_history_note(&entries, &travel);
        self.loaded_history = Some(crate::history_window::LoadedHistory::new(
            &travel.selected_id,
            entries.clone(),
            false,
        ));
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.close_session_tree();
            if let Some(ref text) = travel.editor_text {
                root.set_editor_text(text.clone());
            } else {
                root.set_editor_text(String::new());
            }
        }
        self.sync_ui_root_from_model();
        self.finish_activity_after_rebuild(&entries, &travel);
        self.push_scroll_notice(note);
    }

    pub fn apply_session_tree_fork(
        &mut self,
        child_id: &str,
        entries: Vec<SessionEntry>,
        editor_prefill: Option<String>,
    ) {
        let leaf_id = xylitol_plugin_sdk::session::transcript_leaf_anchor(&entries, None);
        let travel = SessionTreeTravel {
            kind: xylitol_plugin_sdk::session::SessionTreeKind::MessageHistory,
            selected_id: child_id.to_string(),
            leaf_id,
            editor_text: editor_prefill.clone(),
        };
        self.loaded_history = Some(crate::history_window::LoadedHistory::new(
            child_id,
            entries.clone(),
            false,
        ));
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            root.close_session_tree();
            root.set_editor_text(editor_prefill.unwrap_or_default());
        }
        self.sync_ui_root_from_model();
        self.finish_activity_after_rebuild(&entries, &travel);
        self.push_scroll_notice(format!("forked → session {child_id}"));
        self.seed_editor_history_from_entries(&entries);
    }

    pub fn apply_import_session(
        &mut self,
        session_id: &str,
        entries: Vec<SessionEntry>,
        has_more: bool,
    ) {
        self.apply_switched_session(
            session_id,
            entries,
            has_more,
            format!("imported → session {session_id}"),
            |root| {
                root.close_import_confirm();
            },
            false,
        );
    }

    pub fn apply_cli_restored_session(
        &mut self,
        session_id: &str,
        entries: Vec<SessionEntry>,
        has_more: bool,
    ) {
        self.apply_switched_session(
            session_id,
            entries,
            has_more,
            format!("restored → session {session_id}"),
            |_root| {},
            false,
        );
    }

    pub fn apply_resume_session(
        &mut self,
        session_id: &str,
        entries: Vec<SessionEntry>,
        has_more: bool,
    ) {
        self.apply_switched_session(
            session_id,
            entries,
            has_more,
            format!("switched → session {session_id}"),
            |root| {
                root.close_session_tree();
                root.close_session_resume();
            },
            false,
        );
    }

    pub fn apply_new_session(
        &mut self,
        session_id: &str,
        entries: Vec<SessionEntry>,
        has_more: bool,
    ) {
        self.apply_switched_session(
            session_id,
            entries,
            has_more,
            format!("new session → {session_id}"),
            |root| {
                root.close_session_tree();
                root.close_session_resume();
            },
            true,
        );
    }

    pub fn apply_clone_session(
        &mut self,
        session_id: &str,
        entries: Vec<SessionEntry>,
        has_more: bool,
    ) {
        self.apply_switched_session(
            session_id,
            entries,
            has_more,
            format!("cloned → session {session_id}"),
            |root| {
                root.close_session_tree();
            },
            false,
        );
    }

    fn apply_switched_session(
        &mut self,
        session_id: &str,
        entries: Vec<SessionEntry>,
        has_more: bool,
        note: String,
        close_overlays: impl FnOnce(&mut super::super::layout::UiRoot),
        seed_as_new: bool,
    ) {
        let leaf_id = xylitol_plugin_sdk::session::transcript_leaf_anchor(&entries, None);
        let travel = SessionTreeTravel {
            kind: xylitol_plugin_sdk::session::SessionTreeKind::MessageHistory,
            selected_id: session_id.to_string(),
            leaf_id,
            editor_text: None,
        };
        self.loaded_history = Some(crate::history_window::LoadedHistory::new(
            session_id,
            entries.clone(),
            has_more,
        ));
        let buffer_has_more = self
            .loaded_history
            .as_ref()
            .map_or(has_more, |history| history.has_more);
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        self.ui_model.mode_strip = mode_strip_for_entries(&entries);
        crate::history_window::trim_to_window(&mut self.ui_model, buffer_has_more);
        if let Some(root) = self.ui_root.as_ref() {
            let mut root = root.borrow_mut();
            close_overlays(&mut root);
            root.set_editor_text(String::new());
        }
        if !seed_as_new {
            self.seed_editor_history_from_entries(&entries);
        }
        self.sync_ui_root_from_model();
        self.finish_activity_after_rebuild(&entries, &travel);
        self.push_scroll_notice(note);
        self.request_footer_token_refresh();
    }

    pub fn apply_debug_scene(&mut self, load: crate::driver::DebugSceneLoad) {
        let leaf_id = xylitol_plugin_sdk::session::transcript_leaf_anchor(&load.entries, None);
        let travel = SessionTreeTravel {
            kind: xylitol_plugin_sdk::session::SessionTreeKind::MessageHistory,
            selected_id: leaf_id.clone().unwrap_or_else(|| load.session_id.clone()),
            leaf_id,
            editor_text: None,
        };
        self.loaded_history = Some(crate::history_window::LoadedHistory::new(
            &load.session_id,
            load.entries.clone(),
            false,
        ));
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
        self.finish_activity_after_rebuild(&load.entries, &travel);
        self.push_scroll_notice(load.note);
    }

    pub fn apply_fixed_zone_op(&mut self, op: FixedZoneOp) {
        match op {
            FixedZoneOp::Toast => {
                self.push_toast_notice("preview: toast");
            }
            FixedZoneOp::NextTurnCue => {
                if let Some(root) = self.ui_root.as_ref() {
                    root.borrow_mut()
                        .set_status_next_turn_cue(Some("Next turn: preview".into()));
                    self.paint_dirty = true;
                }
            }
            FixedZoneOp::SlotModels => {
                self.mount_models_picker(Vec::new(), None, String::new());
            }
            FixedZoneOp::SlotChoice => {
                let (tx, _rx) = tokio::sync::oneshot::channel();
                self.mount_ask_choice(
                    vec![ChoiceQuestion {
                        id: "preview".into(),
                        label: "Preview".into(),
                        prompt: "fixed-zone preview choice".into(),
                        mode: ChoiceMode::Single,
                        options: vec![ChoiceOption::new("ok", "OK")],
                        allow_other: false,
                    }],
                    tx,
                );
            }
            FixedZoneOp::SlotTree => {
                self.mount_session_tree(Vec::new(), None);
            }
        }
    }

    fn finish_activity_after_rebuild(
        &mut self,
        entries: &[SessionEntry],
        travel: &SessionTreeTravel,
    ) {
        if let Some(root) = self.ui_root.as_ref() {
            root.borrow_mut()
                .apply_activity_after_rebuild(entries, travel);
        }
    }

    pub fn take_load_earlier_request(&mut self) -> bool {
        self.ui_root
            .as_ref()
            .is_some_and(|root| root.borrow_mut().take_load_earlier_request())
    }

    pub fn oldest_loaded_entry_id(&self) -> Option<String> {
        self.loaded_history
            .as_ref()
            .and_then(|h| h.first_entry_id.clone())
    }

    pub fn loaded_history_has_more(&self) -> bool {
        self.loaded_history.as_ref().is_some_and(|h| h.has_more)
    }

    pub fn prepend_history(&mut self, older: Vec<SessionEntry>, has_more: bool) {
        let Some(history) = self.loaded_history.as_mut() else {
            return;
        };
        history.merge_older(older, has_more);
        let entries = history.entries.clone();
        let session_id = history.session_id.clone();
        let has_more = history.has_more;
        let travel = SessionTreeTravel {
            kind: xylitol_plugin_sdk::session::SessionTreeKind::MessageHistory,
            selected_id: session_id,
            leaf_id: xylitol_plugin_sdk::session::transcript_leaf_anchor(&entries, None),
            editor_text: None,
        };
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        self.ui_model.mode_strip = mode_strip_for_entries(&entries);
        crate::history_window::show_notice(&mut self.ui_model, has_more);
        self.sync_ui_root_from_model();
    }

    pub fn expand_history_window(&mut self) {
        let Some(history) = self.loaded_history.as_ref() else {
            return;
        };
        let entries = history.entries.clone();
        let has_more = history.has_more;
        let travel = SessionTreeTravel {
            kind: xylitol_plugin_sdk::session::SessionTreeKind::MessageHistory,
            selected_id: history.session_id.clone(),
            leaf_id: xylitol_plugin_sdk::session::transcript_leaf_anchor(&entries, None),
            editor_text: None,
        };
        rebuild_scrollback_from_travel(&mut self.ui_model, &entries, &travel);
        self.ui_model.mode_strip = mode_strip_for_entries(&entries);
        crate::history_window::show_notice(&mut self.ui_model, has_more);
        self.sync_ui_root_from_model();
    }

    pub fn clear_earlier_hint(&mut self) {
        crate::history_window::show_notice(&mut self.ui_model, false);
        self.sync_ui_root_from_model();
    }
}

fn mode_strip_for_entries(entries: &[SessionEntry]) -> Option<ModeStripState> {
    let mode = latest_plan_mode(entries);
    match mode.mode {
        PlanModeKind::Plan => Some(ModeStripState {
            label: "plan".into(),
        }),
        PlanModeKind::Execute => mode.plan_id.as_ref().map(|id| ModeStripState {
            label: format!("execute · {id}"),
        }),
    }
}
