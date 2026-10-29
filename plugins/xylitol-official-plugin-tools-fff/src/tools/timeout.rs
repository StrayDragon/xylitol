use std::time::Duration;

use xylitol_plugin_sdk::error::XyToolError;

pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
pub const MAX_TIMEOUT_SECS: u64 = 600;

pub fn parse_tool_timeout(arg: Option<i64>) -> Result<Duration, XyToolError> {
    match arg {
        None => Ok(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
        Some(value) if value <= 0 => Err(XyToolError::InvalidArgs(format!(
            "timeout must be a positive integer (got {value})"
        ))),
        Some(value) => Ok(Duration::from_secs((value as u64).min(MAX_TIMEOUT_SECS))),
    }
}
