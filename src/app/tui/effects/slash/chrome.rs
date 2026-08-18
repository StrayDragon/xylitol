//! Chrome / misc slash arms (exit, reload, trust, theme, mcp, copy-last).

use xylitol_tui::Terminal;

use crate::app::core::driver::{XyDriver, XyDriverError};
use crate::app::tui::bridge::UiEntry;
use crate::app::tui::host::HostSession;

use super::super::helpers::note_driver_err;

pub(super) async fn exit<T: Terminal>(session: &mut HostSession<T>) {
    session.request_quit();
}

pub(super) async fn reload<T: Terminal>(session: &mut HostSession<T>) {
    // Agent/bang busy: refuse. Reloading soft-gate is handled in try_reload_input
    // (Enter never queues a second Reload while reload_active).
    if session.run_active() || session.bash_active() {
        session.push_scroll_notice("agent busy — /reload refused");
    } else if session.reload_active() {
        session.push_chrome_toast(crate::app::tui::commands::RELOADING_WAIT_NOTICE);
    } else {
        session.arm_reload();
    }
    let _ = session.render_now();
}

pub(super) async fn trust<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    mode: crate::app::core::driver::ProjectTrustMode,
) {
    if session.is_busy() {
        session.push_scroll_notice("agent busy — /trust refused");
    } else {
        match driver.persist_project_trust(mode) {
            Ok(report) => session.push_scroll_notice(report.message),
            Err(e) => {
                e.log_failure("tui.persist_project_trust");
                session.push_scroll_notice(format!("/trust failed: {e}"));
            }
        }
    }
    let _ = session.render_now();
}

pub(super) async fn history_copy_last<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    // c1110: busy allowed (readonly). Native copy async; OSC52 on host thread.
    match last_assistant_text(&session.ui_model().entries) {
        None => session.push_scroll_notice("no assistant message to copy"),
        Some(text) => {
            let text = text.to_string();
            let n = text.chars().count();
            match driver.copy_text_to_clipboard(&text).await {
                Ok(outcome) => {
                    if let Some(seq) = outcome.pending_osc52.as_deref() {
                        session.emit_clipboard_osc52(seq);
                    }
                    session
                        .push_scroll_notice(format!("Copied last assistant message ({n} chars)"));
                }
                Err(e) => note_driver_err(
                    session,
                    "tui.copy_text_to_clipboard",
                    &e,
                    format!("/history-copy-last failed: {e}"),
                ),
            }
        }
    }
    let _ = session.render_now();
}

pub(super) async fn theme<T: Terminal>(session: &mut HostSession<T>, arg: Option<String>) {
    // c1780: busy Allow chrome theme open/apply.
    match arg.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => session.mount_themes_picker(),
        Some(name) => {
            let resolved = resolve_theme_arg(session, name);
            match resolved {
                Ok(theme_name) => match session.reload_themes(&theme_name) {
                    Ok(()) => {}
                    Err(e) => note_driver_err(
                        session,
                        "tui.reload_themes.slash",
                        &e,
                        format!("/theme failed: {e}"),
                    ),
                },
                Err(e) => note_driver_err(session, "tui.resolve_theme_arg", &e, e.to_string()),
            }
        }
    }
    let _ = session.render_now();
}

pub(super) async fn open_mcp<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    // Prefer sync mount from UiRoot cache; await snapshot only when empty/stale (c1215).
    if session.mcp_cache_usable_for_open() {
        session.mount_mcp_from_cache();
    } else {
        let snap = driver.loaded_resources_snapshot().await;
        session.refresh_loaded_resources_from_snap(snap.clone());
        session.mount_mcp_panel(&snap);
    }
    let _ = session.render_now();
}

pub(super) async fn usage<T: Terminal>(session: &mut HostSession<T>, msg: &'static str) {
    session.push_scroll_notice(msg.to_string());
    let _ = session.render_now();
}

fn last_assistant_text(entries: &[UiEntry]) -> Option<&str> {
    entries.iter().rev().find_map(|e| match e {
        UiEntry::Assistant { text } if !text.trim().is_empty() => Some(text.as_str()),
        _ => None,
    })
}

/// Resolve `/theme` argument to a built-in name (c1115).
fn resolve_theme_arg<T: Terminal>(
    session: &HostSession<T>,
    arg: &str,
) -> Result<String, XyDriverError> {
    match arg.to_ascii_lowercase().as_str() {
        "dark" | "light" => Ok(arg.to_ascii_lowercase()),
        "toggle" | "cycle" => {
            let current = session.theme_preference().unwrap_or("dark");
            let next = if current.eq_ignore_ascii_case("light") {
                "dark"
            } else {
                "light"
            };
            Ok(next.to_string())
        }
        _ => Err(XyDriverError::invalid_input(format!(
            "unknown theme `{arg}` (usage: /theme [dark|light|toggle])"
        ))),
    }
}
