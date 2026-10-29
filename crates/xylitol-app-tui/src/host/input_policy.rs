use xylitol_tui::{InputEvent, Terminal};

use super::super::bridge::UiPhase;
use super::super::commands::{
    BangParse, BusySlashPolicy, PendingBash, PendingSlash, busy_slash_policy,
    busy_slash_refuse_label, parse_bang_command, parse_slash_command_for_surface,
};
use super::super::keybindings::matches_binding;
use super::HostSession;

impl<T: Terminal> HostSession<T> {
    pub(super) fn follow_bottom_on_submit_key(&mut self, input: &InputEvent) {
        let InputEvent::Key(key) = input else {
            return;
        };
        if !matches_binding(key, "tui.input.submit")
            || self
                .ui_root
                .as_ref()
                .is_none_or(|root| root.borrow().slot().is_overlay())
        {
            return;
        }
        self.tui.application_owned_scroll_to_end();
    }

    pub(super) fn try_suppress_stale_esc(&mut self, input: &InputEvent) -> bool {
        if !self.suppress_idle_esc {
            return false;
        }
        if self.is_busy() || self.reload_active {
            self.suppress_idle_esc = false;
            return false;
        }
        if self
            .ui_root
            .as_ref()
            .is_some_and(|root| root.borrow().slot().is_overlay())
        {
            return false;
        }
        let InputEvent::Key(key) = input else {
            self.suppress_idle_esc = false;
            return false;
        };
        if matches_binding(key, "app.interrupt") {
            self.pending.abort = false;
            return true;
        }
        self.suppress_idle_esc = false;
        false
    }

    pub(super) fn try_reload_input(&mut self, input: &InputEvent) -> bool {
        use super::super::commands::RELOADING_WAIT_NOTICE;

        if !self.reload_active {
            return false;
        }
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        let InputEvent::Key(key) = input else {
            return false;
        };
        if root.borrow().slot().is_overlay() {
            return false;
        }
        if matches_binding(key, "app.interrupt") || matches_binding(key, "app.clear") {
            self.request_reload_cancel();
            return true;
        }
        if matches_binding(key, "tui.input.submit") {
            self.push_toast_notice(RELOADING_WAIT_NOTICE);
            return true;
        }
        false
    }

    pub(super) fn try_busy_input(&mut self, input: &InputEvent) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        if !self.is_busy() {
            return false;
        }
        let InputEvent::Key(key) = input else {
            return false;
        };
        if root.borrow().slot().is_overlay() {
            return false;
        }

        if matches_binding(key, "app.interrupt") || matches_binding(key, "app.clear") {
            self.pending.abort = true;
            self.pending.steer = None;
            self.suppress_xy_until_stream_end = true;
            self.ui_model.flush_streaming();
            return true;
        }

        if matches_binding(key, "app.message.dequeue") {
            self.dequeue_to_editor();
            return true;
        }

        if matches_binding(key, "app.message.followUp") {
            let mut root = root.borrow_mut();
            let text = root.editor_text();
            if text.trim().is_empty() {
                return true;
            }
            root.remember_editor_send(text.clone());
            root.set_editor_text(String::new());
            drop(root);
            self.persist_client_history(&text);
            self.ui_model.enqueue_follow_up_strip(text.clone());
            self.pending.follow_up = Some(text);
            self.sync_ui_root_from_model();
            return true;
        }

        if matches_binding(key, "tui.input.submit") {
            let mut root = root.borrow_mut();
            if root.editor_autocomplete_open() {
                let _ = root.confirm_editor_autocomplete();
            }
            let text = root.editor_text();
            if text.trim().is_empty() {
                return true;
            }
            match parse_slash_command_for_surface(&text, self.designing) {
                Some(slash) => {
                    root.set_editor_text(String::new());
                    drop(root);
                    match busy_slash_policy(&slash) {
                        BusySlashPolicy::Allow => {
                            if matches!(slash, PendingSlash::Exit) {
                                self.request_quit();
                            } else {
                                self.pending.slash = Some(slash);
                            }
                            self.sync_ui_root_from_model();
                            return true;
                        }
                        BusySlashPolicy::Reject => {
                            if let PendingSlash::Usage(msg) = slash {
                                self.push_scroll_notice(msg);
                            } else {
                                self.push_scroll_notice(format!(
                                    "agent busy — {} refused",
                                    busy_slash_refuse_label(&slash)
                                ));
                            }
                            self.sync_ui_root_from_model();
                            return true;
                        }
                    }
                }
                None if text.trim().starts_with('/') && looks_like_unknown_slash_command(&text) => {
                    root.set_editor_text(String::new());
                    drop(root);
                    self.push_scroll_notice(format!(
                        "agent busy — unknown command not steered: {}",
                        text.split_whitespace().next().unwrap_or("/")
                    ));
                    self.sync_ui_root_from_model();
                    return true;
                }
                None => {}
            }
            let reject_bang = self.bash_active || self.run_active || self.is_busy();
            if reject_bang {
                match parse_bang_command(&text) {
                    BangParse::NotBang => {}
                    BangParse::Empty { .. } | BangParse::Cmd { .. } => {
                        drop(root);
                        let note = if self.bash_active {
                            "bash already running — wait or Esc to cancel (second ! rejected)"
                        } else {
                            "agent busy — wait or Esc (! command cannot steer; rejected)"
                        };
                        self.push_scroll_notice(note);
                        return true;
                    }
                }
            }
            root.remember_editor_send(text.clone());
            root.set_editor_text(String::new());
            drop(root);
            self.persist_client_history(&text);
            self.ui_model.enqueue_steer_strip(text.clone());
            self.pending.steer = Some(text);
            self.sync_ui_root_from_model();
            return true;
        }

        false
    }

    pub(super) fn try_idle_enter_submit(&mut self, input: &InputEvent) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        if self.run_active || self.ui_model.phase == UiPhase::Busy {
            return false;
        }
        let InputEvent::Key(key) = input else {
            return false;
        };
        if !matches_binding(key, "tui.input.submit") {
            return false;
        }
        let mut root = root.borrow_mut();
        if root.slot().is_overlay() {
            return false;
        }
        if root.editor_autocomplete_open() {
            let _ = root.confirm_editor_autocomplete();
        }
        let text = root.editor_text();
        if text.trim().is_empty() {
            return false;
        }
        self.tui.mark_ao_components_stale();

        if let Some(slash) = parse_slash_command_for_surface(&text, self.designing) {
            root.set_editor_text(String::new());
            drop(root);
            match slash {
                PendingSlash::Exit => {
                    self.request_quit();
                }
                PendingSlash::OpenModels
                | PendingSlash::SetModel(_)
                | PendingSlash::OpenTree
                | PendingSlash::ForkAtLeaf
                | PendingSlash::Compact { .. }
                | PendingSlash::Export { .. }
                | PendingSlash::Import { .. }
                | PendingSlash::SessionDump
                | PendingSlash::OpenSessionResume
                | PendingSlash::SessionNew
                | PendingSlash::SessionClone
                | PendingSlash::SessionName { .. }
                | PendingSlash::Reload
                | PendingSlash::Trust { .. }
                | PendingSlash::HistoryCopyLast
                | PendingSlash::Theme { .. }
                | PendingSlash::OpenMcp
                | PendingSlash::OpenCapabilities
                | PendingSlash::OpenDesigningScenes
                | PendingSlash::SetPlanMode(_)
                | PendingSlash::Usage(_) => {
                    self.pending.slash = Some(slash);
                }
            }
            return true;
        }

        if text.trim().starts_with('/') && looks_like_unknown_slash_command(&text) {
            root.set_editor_text(String::new());
            drop(root);
            self.push_scroll_notice(format!(
                "unknown command: {} {}",
                text.split_whitespace().next().unwrap_or("/"),
                product_slash_hint()
            ));
            return true;
        }

        match parse_bang_command(&text) {
            BangParse::NotBang => {}
            BangParse::Empty { .. } => {
                root.set_editor_text(String::new());
                drop(root);
                self.push_scroll_notice("empty bash command (try !ls or !!ls)");
                return true;
            }
            BangParse::Cmd {
                command,
                exclude_from_context,
            } => {
                root.remember_editor_send(text.clone());
                root.set_editor_text(String::new());
                drop(root);
                self.persist_client_history(&text);
                self.pending.bash = Some(PendingBash {
                    command,
                    exclude_from_context,
                });
                return true;
            }
        }

        root.remember_editor_send(text.clone());
        root.set_editor_text(String::new());
        drop(root);
        self.persist_client_history(&text);
        self.pending.submit = Some(text);
        true
    }

    pub(super) fn try_paste_image(&mut self, input: &InputEvent) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        let InputEvent::Key(key) = input else {
            return false;
        };
        if !matches_binding(key, "app.paste.image") {
            return false;
        }
        if root.borrow().slot().is_overlay() {
            return false;
        }
        self.pending.paste_image = true;
        true
    }

    pub(super) fn try_ctrl_g(&mut self, input: &InputEvent) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        let InputEvent::Key(key) = input else {
            return false;
        };
        if !matches_binding(key, "app.editor.external") {
            return false;
        }
        let mut root = root.borrow_mut();
        if root.slot().is_overlay() {
            return false;
        }

        let prefer_real = {
            #[cfg(test)]
            {
                self.force_real_external_editor
                    || super::super::external_editor::prefer_real_external_editor()
            }
            #[cfg(not(test))]
            {
                super::super::external_editor::prefer_real_external_editor()
            }
        };

        if !prefer_real {
            let chars = root.editor_text().len();
            root.open_external_editor_stub();
            drop(root);
            self.push_scroll_notice(format!(
                "external editor stub (Ctrl+G) · {chars} chars · $EDITOR not spawned"
            ));
            return true;
        }

        let text = root.editor_text();
        drop(root);

        let editor_cmd = {
            #[cfg(test)]
            {
                if let Some(over) = self.external_editor_cmd_override.take() {
                    match over {
                        Ok(cmd) => cmd,
                        Err(err) => {
                            let message = err.to_string();
                            let driver_err = crate::driver::XyDriverError::from(err);
                            driver_err.log_failure("tui.external_editor.resolve");
                            self.push_error_note(message);
                            return true;
                        }
                    }
                } else {
                    match super::super::external_editor::resolve_external_editor_command() {
                        Ok(cmd) => cmd,
                        Err(err) => {
                            let message = err.to_string();
                            let driver_err = crate::driver::XyDriverError::from(err);
                            driver_err.log_failure("tui.external_editor.resolve");
                            self.push_error_note(message);
                            return true;
                        }
                    }
                }
            }
            #[cfg(not(test))]
            {
                match super::super::external_editor::resolve_external_editor_command() {
                    Ok(cmd) => cmd,
                    Err(err) => {
                        let message = err.to_string();
                        let driver_err = crate::driver::XyDriverError::from(err);
                        driver_err.log_failure("tui.external_editor.resolve");
                        self.push_error_note(message);
                        return true;
                    }
                }
            }
        };

        let outcome = self.tui.with_terminal_suspended(|| {
            super::super::external_editor::run_external_editor_process_with_command(
                &editor_cmd,
                &text,
            )
        });
        match outcome {
            Ok(Some(new_text)) => {
                if let Some(root) = self.ui_root.as_ref() {
                    root.borrow_mut().set_editor_text(new_text);
                }
            }
            Ok(None) => {
                log::warn!(
                    target: "xylitol::tui",
                    "tui.external_editor.run failed error.kind=Message error=exited non-zero"
                );
                self.push_error_note(
                    "external editor exited non-zero — keeping original text".to_string(),
                );
            }
            Err(err) => {
                let message = format!("external editor failed: {err}");
                let e = crate::driver::XyDriverError::from(err);
                e.log_failure("tui.external_editor.run");
                self.push_error_note(message);
            }
        }
        true
    }
}

fn looks_like_unknown_slash_command(text: &str) -> bool {
    let first = text.split_whitespace().next().unwrap_or("");
    if !first.starts_with('/') {
        return false;
    }
    first.matches('/').count() <= 1
}

fn product_slash_hint() -> String {
    let names = crate::product_commands::product_slash_commands();
    let joined = names
        .iter()
        .map(|c| format!("/{}", c.name))
        .collect::<Vec<_>>()
        .join(", ");
    format!("(try {joined})")
}
