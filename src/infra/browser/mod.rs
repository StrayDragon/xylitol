//! Open a URL in the system default browser.

/// Browser launch failures (crate-private; not `Xy*`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct BrowserError(pub String);

impl From<&str> for BrowserError {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for BrowserError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Open a URL in the default system browser.
pub fn open_browser(url: &str) -> Result<(), BrowserError> {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .status()
    } else {
        // Linux: try xdg-open, then sensible-browser
        std::process::Command::new("xdg-open")
            .arg(url)
            .status()
            .or_else(|_| {
                std::process::Command::new("sensible-browser")
                    .arg(url)
                    .status()
            })
    };

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(format!("failed to open browser for {url}").into()),
        Err(e) => Err(format!("could not launch browser: {e}").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "opens the system browser; run manually with -- --ignored"]
    fn test_open_browser_invalid_url_no_panic() {
        // Should not panic — may fail on CI without a browser
        let _ = open_browser("https://example.com");
    }
}
