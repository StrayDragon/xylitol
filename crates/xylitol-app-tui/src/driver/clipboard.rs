use super::XyDriverError;
use super::types::ClipboardCopyOutcome;

pub async fn copy_text_to_clipboard(text: String) -> Result<ClipboardCopyOutcome, XyDriverError> {
    let plan = crate::clipboard::plan_clipboard_copy_async(text)
        .await
        .map_err(XyDriverError::from)?;
    if !plan.will_succeed() {
        return Err(XyDriverError::io(plan.failure_message()));
    }
    Ok(ClipboardCopyOutcome {
        pending_osc52: plan.osc52_sequence,
    })
}

pub async fn stage_clipboard_image() -> Result<Option<std::path::PathBuf>, XyDriverError> {
    let image = tokio::task::spawn_blocking(crate::clipboard::read_clipboard_image)
        .await
        .map_err(|e| XyDriverError::io(format!("clipboard image task failed: {e}")))?
        .map_err(XyDriverError::from)?;
    let Some(image) = image else {
        return Ok(None);
    };
    let path = tokio::task::spawn_blocking(move || {
        crate::clipboard::write_clipboard_image_temp(&image.bytes, &image.mime_type)
    })
    .await
    .map_err(|e| XyDriverError::io(format!("clipboard image write task failed: {e}")))?
    .map_err(XyDriverError::from)?;
    Ok(Some(path))
}

pub async fn read_clipboard_text() -> Result<Option<String>, XyDriverError> {
    tokio::task::spawn_blocking(crate::clipboard::read_clipboard_text)
        .await
        .map_err(|e| XyDriverError::io(format!("clipboard text task failed: {e}")))?
        .map_err(XyDriverError::from)
}
