//! Session import confirm decision.

use xylitol_tui::Terminal;

use crate::app::core::dispatch::{DispatchOutcome, dispatch};
use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;
use crate::app::tui::layout::ImportConfirmDecision;
use crate::protocol::Command;

use super::super::helpers::{SwitchRebuildKind, switch_and_rebuild_transcript};

pub(super) async fn decide<T: Terminal>(session: &mut HostSession<T>, driver: &mut dyn XyDriver) {
    let Some(decision) = session.take_pending_import_decision() else {
        return;
    };
    match decision {
        ImportConfirmDecision::Rejected => {
            session.push_scroll_notice("Import cancelled");
            session.close_import_confirm();
        }
        ImportConfirmDecision::Accepted { path } => {
            log::info!(target: "xylitol::tui", "ImportJsonl path={}", path);
            match dispatch(
                driver,
                Command::ImportJsonl {
                    id: None,
                    input_path: path.clone(),
                },
            )
            .await
            {
                Ok(DispatchOutcome::NewSession(id)) => {
                    switch_and_rebuild_transcript(session, driver, &id, SwitchRebuildKind::Import)
                        .await;
                }
                Ok(_) => {
                    session.push_scroll_notice("import complete");
                    session.close_import_confirm();
                }
                // dispatch already logs error.kind
                Err(e) => {
                    session.push_scroll_notice(format!("/session-import failed: {e}"));
                    session.close_import_confirm();
                }
            }
        }
    }
    let _ = session.render_now();
}
