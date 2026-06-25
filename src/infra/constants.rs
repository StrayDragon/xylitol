//! Project-wide constants.
//!
//! Centralizes all magic strings and URLs to avoid hardcoding throughout the codebase.

/// Base URL for the version-check API.
///
/// Derived from pi's update service (see <https://pi.dev>).
pub const UPDATE_API_BASE_URL: &str = "https://pi.dev/api";

/// Endpoint path for checking the latest version.
pub const LATEST_VERSION_ENDPOINT: &str = "/latest-version";

/// Full URL for the version-check endpoint.
pub const VERSION_CHECK_URL: &str =
    const_format::concatcp!(UPDATE_API_BASE_URL, LATEST_VERSION_ENDPOINT);

/// Environment variable to skip version checking.
pub const ENV_SKIP_VERSION_CHECK: &str = "PI_SKIP_VERSION_CHECK";

/// Environment variable to run in offline mode.
pub const ENV_OFFLINE: &str = "PI_OFFLINE";

/// User-Agent string for HTTP requests.
pub const HTTP_USER_AGENT: &str = "xylitol";
