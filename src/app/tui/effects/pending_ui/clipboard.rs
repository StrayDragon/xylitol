//! Ctrl+V clipboard paste (image then text fallback).

use xylitol_tui::Terminal;

use crate::app::core::driver::XyDriver;
use crate::app::tui::host::HostSession;

use super::super::helpers::note_driver_err_styled;

pub(super) async fn take_paste_image<T: Terminal>(
    session: &mut HostSession<T>,
    driver: &mut dyn XyDriver,
) {
    if !session.take_paste_image() {
        return;
    }
    log::info!(target: "xylitol::tui", "XyDriver::stage_clipboard_image");
    let image_outcome = driver.stage_clipboard_image().await;
    match image_outcome {
        Ok(Some(path)) => {
            let insert = path.display().to_string();
            if let Some(root) = session.ui_root() {
                root.borrow_mut().insert_editor_text_at_cursor(&insert);
            }
        }
        Ok(None) | Err(_) => {
            // c1156 / pi: no image (or read failed) → try system clipboard text.
            if let Err(e) = &image_outcome {
                e.log_failure("tui.stage_clipboard_image");
                log::info!(
                    target: "xylitol::tui",
                    "clipboard image unavailable, trying text: {e}"
                );
            }
            match driver.read_clipboard_text().await {
                Ok(Some(text)) if !text.is_empty() => {
                    if let Some(root) = session.ui_root() {
                        root.borrow_mut().insert_editor_text_at_cursor(&text);
                    }
                }
                Ok(_) => {
                    session.push_error_note("clipboard: no image or text");
                }
                Err(e) => {
                    note_driver_err_styled(
                        session,
                        "tui.read_clipboard_text",
                        &e,
                        format!("clipboard: {e}"),
                    );
                }
            }
        }
    }
    let _ = session.render_now();
}
