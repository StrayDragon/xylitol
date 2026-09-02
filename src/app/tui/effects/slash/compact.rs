//! `/session-compact` slash arm.

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::XyDriver;
use crate::app::tui::bridge::{CompactionBlockStatus, UiEntry};
use crate::app::tui::host::HostSession;
use crate::protocol::Command;

pub(super) async fn run<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
    instructions: Option<String>,
) {
    match dispatch(driver, Command::Compact { instructions }).await {
        Ok(DispatchOutcome::Compacted(did)) => {
            if did {
                append_compaction_from_session(session, driver).await;
            } else {
                session.push_scroll_notice("session unchanged (nothing to compact)");
            }
            super::super::refresh_footer_tokens(session, driver).await;
        }
        Ok(_) => {
            append_compaction_from_session(session, driver).await;
            super::super::refresh_footer_tokens(session, driver).await;
        }
        Err(e) => session.push_scroll_notice(format!("/session-compact failed: {e}")),
    }
    let _ = session.render_now();
}

/// After slash/force compact, surface the latest CompactionEntry as a collapsed block.
async fn append_compaction_from_session<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &dyn XyDriver,
) {
    let Ok(entries) = driver.get_messages().await else {
        session.push_scroll_notice("session compacted");
        return;
    };
    let Some(comp) = entries.iter().rev().find_map(|e| match e {
        crate::protocol::session::SessionEntry::Compaction(c) => Some(c),
        _ => None,
    }) else {
        session.push_scroll_notice("session compacted");
        return;
    };
    // Avoid duplicate if CompactionEnd already arrived via turn stream / tee.
    let already = session.ui_model().entries.iter().any(|e| {
        matches!(
            e,
            UiEntry::Compaction {
                status: CompactionBlockStatus::Complete,
                tokens_before,
                ..
            } if *tokens_before == comp.tokens_before
        )
    });
    if already {
        return;
    }
    session.ui_model_mut().entries.push(UiEntry::Compaction {
        status: CompactionBlockStatus::Complete,
        summary: comp.summary.clone(),
        tokens_before: comp.tokens_before,
        detail: None,
    });
    session.sync_ui_root_from_model();
}
