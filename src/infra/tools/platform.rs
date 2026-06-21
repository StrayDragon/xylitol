//! Platform detection and GitHub release asset name mapping.

pub(crate) struct ToolDownloadConfig {
    pub repo: &'static str,
    pub binary_name: &'static str,
    pub tag_prefix: &'static str,
    pub get_asset_name: fn(version: &str, config: &ToolDownloadConfig) -> Option<String>,
}

/// Get the download configuration for a known tool.
pub(crate) fn tool_config(name: &str) -> Option<ToolDownloadConfig> {
    match name {
        "fd" => Some(ToolDownloadConfig {
            repo: "sharkdp/fd",
            binary_name: "fd",
            tag_prefix: "v",
            get_asset_name: |version, _config| {
                let arch = if cfg!(target_arch = "aarch64") {
                    "aarch64"
                } else {
                    "x86_64"
                };
                let os = if cfg!(target_os = "macos") {
                    "apple-darwin"
                } else if cfg!(target_os = "linux") {
                    "unknown-linux-gnu"
                } else if cfg!(target_os = "windows") {
                    "pc-windows-msvc"
                } else {
                    return None;
                };
                let ext = if cfg!(target_os = "windows") {
                    "zip"
                } else {
                    "tar.gz"
                };
                Some(format!("fd-v{version}-{arch}-{os}.{ext}"))
            },
        }),
        "rg" => Some(ToolDownloadConfig {
            repo: "BurntSushi/ripgrep",
            binary_name: "rg",
            tag_prefix: "",
            get_asset_name: |version, _config| {
                let arch = if cfg!(target_arch = "aarch64") {
                    "aarch64"
                } else {
                    "x86_64"
                };
                let os = if cfg!(target_os = "macos") {
                    "apple-darwin"
                } else if cfg!(target_os = "linux") {
                    "unknown-linux-musl"
                } else if cfg!(target_os = "windows") {
                    "pc-windows-msvc"
                } else {
                    return None;
                };
                let ext = if cfg!(target_os = "windows") {
                    "zip"
                } else {
                    "tar.gz"
                };
                Some(format!("ripgrep-{version}-{arch}-{os}.{ext}"))
            },
        }),
        _ => None,
    }
}
