//! Open a URL in the system default browser.

#![allow(dead_code)]

/// Open a URL in the default system browser.
pub fn open_browser(url: &str) -> Result<(), String> {
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
            .or_else(|_| std::process::Command::new("sensible-browser").arg(url).status())
    };

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(format!("failed to open browser for {url}")),
        Err(e) => Err(format!("could not launch browser: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_browser_invalid_url_no_panic() {
        // Should not panic — may fail on CI without a browser
        let _ = open_browser("https://example.com");
    }
}
