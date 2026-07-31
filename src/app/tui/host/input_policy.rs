//! Busy/idle input policy for HostSession::step (c1170 / ath12).

use xylitol_tui::{InputEvent, Terminal};

use super::super::bridge::UiPhase;
use super::super::commands::{
    BangParse, BusySlashPolicy, PendingBash, PendingSlash, busy_slash_policy,
    busy_slash_refuse_label, parse_bang_command, parse_slash_command,
};
use super::super::keybindings::matches_binding;
use super::HostSession;

impl<T: Terminal> HostSession<T> {
    /// Drop Esc backlog after abort while idle so the next bang is not cancelled
    /// at submit. Never suppress Esc while busy (second bang must stay abortable).
    pub(super) fn try_suppress_stale_esc(&mut self, input: &InputEvent) -> bool {
        if !self.suppress_idle_esc {
            return false;
        }
        // New busy work owns Esc for abort — clear suppress and do not consume.
        if self.is_busy() {
            self.suppress_idle_esc = false;
            return false;
        }
        let InputEvent::Key(key) = input else {
            // Paste / other input: user moved on; clear suppress.
            self.suppress_idle_esc = false;
            return false;
        };
        if matches_binding(key, "app.interrupt") {
            self.pending.abort = false;
            return true;
        }
        // Any other key (typing the next `!cmd`) ends idle Esc suppress.
        self.suppress_idle_esc = false;
        false
    }

    /// Busy Enter / Alt+Enter / Esc / Alt+Up (c480). Returns true when consumed.
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
            // Drop untaken local steer so loop does not re-enqueue after abort.
            self.pending.steer = None;
            // c720 / c1595: flush partial into entries immediately so late Xy cannot
            // revive UI; drain_pending still calls XyDriver::abort + note_user_abort.
            // c1570: Ctrl+C (app.clear) shares this latch while busy — MUST NOT quit.
            self.suppress_xy_until_stream_end = true;
            self.ui_model.flush_streaming();
            return true;
        }

        if matches_binding(key, "app.message.dequeue") {
            let queued = self.ui_model.take_queued_for_editor();
            if queued.is_empty() {
                return true;
            }
            let mut root = root.borrow_mut();
            let current = root.editor_text();
            let mut parts = queued;
            if !current.trim().is_empty() {
                parts.push(current);
            }
            root.set_editor_text(parts.join("\n\n"));
            drop(root);
            self.pending.steer = None;
            self.pending.follow_up = None;
            self.pending.dequeue = true;
            self.sync_ui_root_from_model();
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
            // pi queue strip: dim Follow-up: above status — not a scrollback [steer] wall.
            self.ui_model.enqueue_follow_up_strip(text.clone());
            self.pending.follow_up = Some(text);
            self.sync_ui_root_from_model();
            return true;
        }

        if matches_binding(key, "tui.input.submit") {
            let mut root = root.borrow_mut();
            let text = root.editor_text();
            if text.trim().is_empty() {
                return true;
            }
            match parse_slash_command(&text) {
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
            // c669 / ati32: hard-reject bang while bash_active or agent busy
            // (must not steer literal `!cmd`).
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
            self.ui_model.enqueue_steer_strip(text.clone());
            self.pending.steer = Some(text);
            self.sync_ui_root_from_model();
            return true;
        }

        false
    }

    /// Idle Enter: slash or prompt submit. Steer / Alt+Enter are busy-only.
    /// Returns true when Enter was consumed.
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
        // Product steals Enter before Editor; when the slash/fuzzy popup is open,
        // apply the highlighted item first (pi select.confirm), then parse.
        // Otherwise `/new` stays literal → unknown while `session-new` is selected.
        if root.editor_autocomplete_open() {
            let _ = root.confirm_editor_autocomplete();
        }
        let text = root.editor_text();
        if text.trim().is_empty() {
            return false;
        }

        if let Some(slash) = parse_slash_command(&text) {
            root.set_editor_text(String::new());
            drop(root);
            match slash {
                PendingSlash::Exit => {
                    self.request_quit();
                }
                PendingSlash::OpenModels
                | PendingSlash::SetModel(_)
                | PendingSlash::DebugScene(_)
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
                "unknown command: {} (try /exit, /model, /theme, /mcp, /session, /session-resume, /session-new, /session-clone, /session-name, /session-tree, /session-fork, /session-compact, /session-export, /session-import, /reload, /trust, /history-copy-last)",
                text.split_whitespace().next().unwrap_or("/")
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
        self.pending.submit = Some(text);
        true
    }

    /// Ctrl+V / `app.paste.image`: stage clipboard image via XyDriver (c1155).
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

    /// Ctrl+G: external editor (c650 / ati17) — TTY real path or harness stub.
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
                if let Some(over) = self.external_editor_cmd_override.clone() {
                    match over {
                        Ok(cmd) => cmd,
                        Err(msg) => {
                            let err =
                                crate::app::core::driver::XyDriverError::invalid_input(msg.clone());
                            err.log_failure("tui.external_editor.resolve");
                            self.push_error_note(msg);
                            return true;
                        }
                    }
                } else {
                    match super::super::external_editor::resolve_external_editor_command() {
                        Ok(cmd) => cmd,
                        Err(msg) => {
                            let err =
                                crate::app::core::driver::XyDriverError::invalid_input(msg.clone());
                            err.log_failure("tui.external_editor.resolve");
                            self.push_error_note(msg);
                            return true;
                        }
                    }
                }
            }
            #[cfg(not(test))]
            {
                match super::super::external_editor::resolve_external_editor_command() {
                    Ok(cmd) => cmd,
                    Err(msg) => {
                        let err =
                            crate::app::core::driver::XyDriverError::invalid_input(msg.clone());
                        err.log_failure("tui.external_editor.resolve");
                        self.push_error_note(msg);
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
                let e = crate::app::core::driver::XyDriverError::from_opaque(err.clone());
                e.log_failure("tui.external_editor.run");
                self.push_error_note(format!("external editor failed: {err}"));
            }
        }
        // Soft render via step(); suspend kept previous_lines for differential
        // (no full-screen clear — inline TUI preserves scrollback above).
        true
    }
}

/// True when idle Enter should treat leading `/` as an unknown slash (not a filesystem path).
///
/// Absolute paths like `/tmp/xylitol-paste-….png` (c1155 / pi) contain another `/` after the
/// root slash; slash commands are a single token (`/model`, `/session-new`).
fn looks_like_unknown_slash_command(text: &str) -> bool {
    let first = text.split_whitespace().next().unwrap_or("");
    if !first.starts_with('/') {
        return false;
    }
    // `/tmp/…`, `/home/…`, `//unc` → prompt text, not a slash command.
    first.matches('/').count() <= 1
}
