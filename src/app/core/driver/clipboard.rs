//! Client-local clipboard semantics shared by every [`XyDriver`] implementation.
//!
//! Both the in-process driver and the remote driver run in the TUI process
//! (the remote driver is a client of the Host), so platform clipboard tools /
//! OSC 52 act on the TUI machine. MUST NOT route through the Host (D1).

use super::XyDriverError;
use super::types::ClipboardCopyOutcome;

pub(crate) async fn copy_text_to_clipboard(
    text: String,
) -> Result<ClipboardCopyOutcome, XyDriverError> {
    let plan = crate::infra::clipboard::plan_clipboard_copy_async(text)
        .await
        .map_err(XyDriverError::from)?;
    if !plan.will_succeed() {
        return Err(XyDriverError::io(plan.failure_message()));
    }
    Ok(ClipboardCopyOutcome {
        pending_osc52: plan.osc52_sequence,
    })
}

pub(crate) async fn stage_clipboard_image() -> Result<Option<std::path::PathBuf>, XyDriverError> {
    let image = tokio::task::spawn_blocking(crate::infra::clipboard::read_clipboard_image)
        .await
        .map_err(|e| XyDriverError::io(format!("clipboard image task failed: {e}")))?
        .map_err(XyDriverError::from)?;
    let Some(image) = image else {
        return Ok(None);
    };
    let path = tokio::task::spawn_blocking(move || {
        crate::infra::clipboard::write_clipboard_image_temp(&image.bytes, &image.mime_type)
    })
    .await
    .map_err(|e| XyDriverError::io(format!("clipboard image write task failed: {e}")))?
    .map_err(XyDriverError::from)?;
    Ok(Some(path))
}

pub(crate) async fn read_clipboard_text() -> Result<Option<String>, XyDriverError> {
    tokio::task::spawn_blocking(crate::infra::clipboard::read_clipboard_text)
        .await
        .map_err(|e| XyDriverError::io(format!("clipboard text task failed: {e}")))?
        .map_err(XyDriverError::from)
}
