//! Print mode — non-interactive, streaming stdout output.

use crate::app::core::driver::{XyDriver, XyDriverError};

pub use xylitol_app_print::{format_tool_end_line, format_tool_start_lines, render_stream};

/// Run the agent in print mode with the given prompt.
pub(crate) async fn run_print(
    driver: &mut dyn XyDriver,
    prompt: &str,
    session_id: &str,
) -> Result<(), XyDriverError> {
    let mut stream = driver.run(prompt).await;
    xylitol_app_print::run_print(&mut stream, session_id)
        .await
        .map_err(|e| XyDriverError::message(e.to_string()))
}
