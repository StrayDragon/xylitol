//! Tool downloader — auto-downloads fd and rg binaries from GitHub Releases.
#![allow(dead_code)]

mod platform;

use std::path::PathBuf;

/// Ensure a tool (fd or rg) is available, downloading if necessary.
pub async fn ensure_tool(name: &str) -> Option<PathBuf> {
    // Check if already on PATH
    if tool_on_path(name) {
        return Some(PathBuf::from(name));
    }

    // Check offline mode
    if std::env::var("PI_OFFLINE")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return None;
    }

    // Download
    let config = platform::tool_config(name)?;
    let version = get_latest_version(config.repo).await?;
    let asset_name = (config.get_asset_name)(&version, &config)?;
    let download_url = format!(
        "https://github.com/{}/releases/download/{}{}/{}",
        config.repo, config.tag_prefix, version, asset_name
    );

    let bin_dir = get_bin_dir()?;
    tokio::fs::create_dir_all(&bin_dir).await.ok()?;

    let archive_path = bin_dir.join(&asset_name);

    // Download
    let response = reqwest::get(&download_url).await.ok()?;
    let bytes = response.bytes().await.ok()?;
    tokio::fs::write(&archive_path, &bytes).await.ok()?;

    // Extract
    let binary_path = extract_binary(&archive_path, config.binary_name, &bin_dir).await?;

    // Remove archive
    let _ = tokio::fs::remove_file(&archive_path).await;

    Some(binary_path)
}

fn tool_on_path(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn get_latest_version(repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let client = reqwest::Client::builder()
        .user_agent("xylitol-coding-agent")
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    let data: serde_json::Value = resp.json().await.ok()?;
    let tag = data["tag_name"].as_str()?;
    Some(tag.trim_start_matches('v').to_string())
}

async fn extract_binary(
    archive_path: &std::path::Path,
    binary_name: &str,
    bin_dir: &std::path::Path,
) -> Option<PathBuf> {
    let archive_bytes = tokio::fs::read(archive_path).await.ok()?;
    let ext = archive_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let binary_path = if ext == "zip" {
        extract_zip(&archive_bytes, binary_name, bin_dir)?
    } else {
        extract_tar_gz(&archive_bytes, binary_name, bin_dir)?
    };

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        let _ = std::fs::set_permissions(&binary_path, perms);
    }

    Some(binary_path)
}

fn extract_tar_gz(data: &[u8], binary_name: &str, bin_dir: &std::path::Path) -> Option<PathBuf> {
    let decoder = flate2::read::GzDecoder::new(data);
    let mut archive = tar::Archive::new(decoder);
    let tmp_dir = bin_dir.join(format!("extract_{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).ok()?;

    archive.unpack(&tmp_dir).ok()?;

    // Find the binary recursively
    let binary = find_binary_recursive(&tmp_dir, binary_name)?;
    let dest = bin_dir.join(binary_name);
    std::fs::rename(&binary, &dest).ok()?;
    let _ = std::fs::remove_dir_all(&tmp_dir);
    Some(dest)
}

fn extract_zip(data: &[u8], binary_name: &str, bin_dir: &std::path::Path) -> Option<PathBuf> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data)).ok()?;
    let tmp_dir = bin_dir.join(format!("extract_zip_{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir).ok()?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).ok()?;
        if let Some(name) = file.name().split('/').next_back()
            && name == binary_name
        {
            let dest = tmp_dir.join(name);
            let mut out = std::fs::File::create(&dest).ok()?;
            std::io::copy(&mut file, &mut out).ok()?;
            let final_dest = bin_dir.join(binary_name);
            std::fs::rename(&dest, &final_dest).ok()?;
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return Some(final_dest);
        }
    }
    let _ = std::fs::remove_dir_all(&tmp_dir);
    None
}

fn find_binary_recursive(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|s| s.to_str()) == Some(name) {
            return Some(path);
        }
        if path.is_dir()
            && let Some(found) = find_binary_recursive(&path, name)
        {
            return Some(found);
        }
    }
    None
}

fn get_bin_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".xylitol").join("bin"))
}
