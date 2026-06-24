//! Project-wide constants.
//!
//! Centralizes all magic strings and URLs to avoid hardcoding throughout the codebase.
//!
//! ## Configuration
//!
//! The update-related constants below are placeholders. To use version checking,
//! set the `XYLITOL_UPDATE_URL` environment variable to your update service URL,
//! or modify these constants to point to your actual service.
//!
//! Inspired by pi's update service architecture (see <https://pi.dev>).

/// Base URL for the version-check API.
///
/// **Placeholder**: Replace with your actual update service URL.
/// Can be overridden via `XYLITOL_UPDATE_URL` environment variable.
pub const UPDATE_API_BASE_URL: &str = "https://api.example.com/v1";

/// Endpoint path for checking the latest version.
pub const LATEST_VERSION_ENDPOINT: &str = "/latest-version";

/// Full URL for the version-check endpoint.
///
/// **Placeholder**: This is a concatenation of base URL and endpoint.
/// Set `XYLITOL_UPDATE_URL` to override the entire URL.
pub const VERSION_CHECK_URL: &str =
    const_format::concatcp!(UPDATE_API_BASE_URL, LATEST_VERSION_ENDPOINT);

/// Environment variable to override the update URL.
pub const ENV_UPDATE_URL: &str = "XYLITOL_UPDATE_URL";

/// Environment variable to skip version checking.
pub const ENV_SKIP_VERSION_CHECK: &str = "XYLITOL_SKIP_VERSION_CHECK";

/// Environment variable to run in offline mode.
pub const ENV_OFFLINE: &str = "XYLITOL_OFFLINE";

/// User-Agent string for HTTP requests.
pub const HTTP_USER_AGENT: &str = "xylitol";
