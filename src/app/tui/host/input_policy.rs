//! Busy/idle input policy for HostSession::step (c1170 / ath12).

use xylitol_tui::{InputEvent, Terminal, matches_key_event};

use super::super::bridge::UiPhase;
use super::super::commands::{
    BangParse, PendingBash, PendingSlash, parse_bang_command, parse_slash_command,
};
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
        if matches_key_event(key, "escape") {
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

        if matches_key_event(key, "escape") {
            self.pending.abort = true;
            // Drop untaken local steer so loop does not re-enqueue after abort.
            self.pending.steer = None;
            // c720 / 1A: arm suppress immediately so late Xy cannot revive UI before
            // drain_pending calls Driver::abort (token waste / fake busy).
            self.suppress_xy_until_stream_end = true;
            self.ui_model.clear_streaming_buffers();
            return true;
        }

        if matches_key_event(key, "alt+up") {
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

        if matches_key_event(key, "alt+enter") {
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

        if matches_key_event(key, "enter") {
            let mut root = root.borrow_mut();
            let text = root.editor_text();
            if text.trim().is_empty() {
                return true;
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
                        self.push_system_note(note);
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
        if !matches_key_event(key, "enter") {
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
                | PendingSlash::Compact
                | PendingSlash::Export { .. }
                | PendingSlash::Import { .. }
                | PendingSlash::SessionDump
                | PendingSlash::OpenSessionResume
                | PendingSlash::SessionNew
                | PendingSlash::SessionClone
                | PendingSlash::SessionName { .. }
                | PendingSlash::Usage(_) => {
                    self.pending.slash = Some(slash);
                }
            }
            return true;
        }

        if text.trim().starts_with('/') {
            root.set_editor_text(String::new());
            drop(root);
            self.push_system_note(format!(
                "unknown command: {} (try /exit, /model, /session, /session-resume, /session-new, /session-clone, /session-name, /session-tree, /session-fork, /session-compact, /session-export, /session-import)",
                text.split_whitespace().next().unwrap_or("/")
            ));
            return true;
        }

        match parse_bang_command(&text) {
            BangParse::NotBang => {}
            BangParse::Empty { .. } => {
                root.set_editor_text(String::new());
                drop(root);
                self.push_system_note("empty bash command (try !ls or !!ls)");
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

    /// Ctrl+G: external editor (c650 / ati17) — TTY real path or harness stub.
    pub(super) fn try_ctrl_g(&mut self, input: &InputEvent) -> bool {
        let Some(root) = self.ui_root.as_ref() else {
            return false;
        };
        let InputEvent::Key(key) = input else {
            return false;
        };
        if !matches_key_event(key, "ctrl+g") {
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
            self.push_system_note(format!(
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
                            self.push_error_note(msg);
                            return true;
                        }
                    }
                } else {
                    match super::super::external_editor::resolve_external_editor_command() {
                        Ok(cmd) => cmd,
                        Err(msg) => {
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
                self.push_error_note(
                    "external editor exited non-zero — keeping original text".to_string(),
                );
            }
            Err(err) => {
                self.push_error_note(format!("external editor failed: {err}"));
            }
        }
        // Soft render via step(); suspend kept previous_lines for differential
        // (no full-screen clear — inline TUI preserves scrollback above).
        true
    }
}
