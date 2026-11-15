use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::args::PluginsAction;

const DESCRIBE_TIMEOUT: Duration = Duration::from_secs(2);
const PLUGIN_PREFIX: &str = "xylitol-";

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PluginsSettings {
    paths: Vec<String>,
    path_from_env: bool,
    allow: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginManifest {
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    requires_host: bool,
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    default_bind: Option<String>,
    #[serde(default)]
    default_port: Option<u16>,
    #[serde(default)]
    auth: Option<String>,
}

#[derive(Debug, Clone)]
struct PluginEntry {
    name: String,
    path: PathBuf,
    manifest: Option<PluginManifest>,
}

pub fn run_plugins(action: PluginsAction) -> Result<(), Box<dyn std::error::Error>> {
    let settings = load_settings();
    let entries = discover(&settings);
    match action {
        PluginsAction::List => {
            if entries.is_empty() {
                println!("no plugins discovered");
                return Ok(());
            }
            for entry in entries {
                let mode = if entry
                    .manifest
                    .as_ref()
                    .is_some_and(|manifest| manifest.requires_host)
                {
                    "host"
                } else {
                    "local"
                };
                println!("{}\t{}\t{}", entry.name, mode, entry.path.display());
            }
        }
        PluginsAction::Describe { name } => {
            let entry = entries
                .into_iter()
                .find(|entry| entry.name == name)
                .ok_or_else(|| format!("plugin not found: {name}"))?;
            let Some(manifest) = entry.manifest else {
                return Err(format!("plugin {name} has no valid --describe manifest").into());
            };
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        }
    }
    Ok(())
}

pub async fn dispatch_external(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let Some(name) = args.first().cloned() else {
        return Err("missing plugin name".into());
    };
    validate_name(&name)?;
    let settings = load_settings();
    if !settings.allow.is_empty() && !settings.allow.iter().any(|allowed| allowed == &name) {
        return Err(format!("plugin {name} is not in plugins.allow").into());
    }
    let entries = discover(&settings);
    let entry = entries
        .into_iter()
        .find(|entry| entry.name == name)
        .ok_or_else(|| format!("plugin not found: {name}"))?;
    let requires_host = entry
        .manifest
        .as_ref()
        .is_some_and(|manifest| manifest.requires_host);
    let mut command = Command::new(&entry.path);
    command.args(&args[1..]);
    command.env("XYLITOL_PLUGIN_NAME", &name);
    if requires_host {
        #[cfg(feature = "server")]
        {
            let url = xylitol_kernel::attach::resolve_attach_url(None, None);
            xylitol_kernel::attach::ensure_host_from_config(&url, None)
                .await
                .map_err(|error| format!("ensure host failed: {error}"))?;
            command.env("XYLITOL_HOST_URL", &url);
            let token = xylitol_kernel::attach::registration_token(&url).or_else(|| {
                xylitol_serve::load_or_create_credential(&xylitol_serve::default_credential_path())
                    .ok()
            });
            if let Some(token) = token {
                command.env("XYLITOL_HOST_TOKEN", token);
            }
        }
        #[cfg(not(feature = "server"))]
        {
            return Err(
                format!("plugin {name} requires Host, but the server feature is disabled").into(),
            );
        }
    }
    let status = command.status()?;
    if !status.success() {
        return Err(format!("plugin {name} exited with {status}").into());
    }
    Ok(())
}

pub fn external_command_name(args: &[String]) -> Option<String> {
    let settings = load_settings();
    external_command_name_in_dirs(args, &search_dirs(&settings))
}

fn external_command_name_in_dirs(args: &[String], dirs: &[PathBuf]) -> Option<String> {
    let first = args.get(1)?;
    if first.starts_with('-') {
        return None;
    }
    if matches!(
        first.as_str(),
        "tui" | "print" | "config" | "sessions" | "plugins" | "serve"
    ) {
        return None;
    }
    validate_name(first).ok()?;
    find_plugin_binary_in_dirs(first, dirs).map(|_| first.clone())
}

fn find_plugin_binary_in_dirs(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let binary = format!("{PLUGIN_PREFIX}{name}");
    let mut seen = HashSet::new();
    for dir in dirs {
        if !seen.insert(dir.clone()) {
            continue;
        }
        let path = dir.join(&binary);
        if is_executable(&path) {
            return Some(path);
        }
    }
    None
}

fn load_settings() -> PluginsSettings {
    xylitol_kernel::config::loader::load_app_config(None)
        .ok()
        .and_then(|config| config.sections.get("plugins").cloned())
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

fn discover(settings: &PluginsSettings) -> Vec<PluginEntry> {
    discover_dirs(&search_dirs(settings))
}

fn search_dirs(settings: &PluginsSettings) -> Vec<PathBuf> {
    let mut dirs = vec![plugin_dir()];
    dirs.extend(settings.paths.iter().map(PathBuf::from));
    if settings.path_from_env
        && let Some(path) = std::env::var_os("PATH")
    {
        dirs.extend(std::env::split_paths(&path));
    }
    dirs
}

fn discover_dirs(dirs: &[PathBuf]) -> Vec<PluginEntry> {
    let mut seen_dirs = HashSet::new();
    let mut entries: Vec<PluginEntry> = Vec::new();
    for dir in dirs {
        if !seen_dirs.insert(dir.clone()) {
            continue;
        }
        let Ok(read) = std::fs::read_dir(dir) else {
            continue;
        };
        for item in read.flatten() {
            let path = item.path();
            let Some(name) = plugin_name(&path) else {
                continue;
            };
            if !is_executable(&path) {
                continue;
            }
            if entries.iter().any(|entry| entry.name == name) {
                continue;
            }
            let manifest = describe(&path);
            entries.push(PluginEntry {
                name,
                path,
                manifest,
            });
        }
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn plugin_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xylitol")
        .join("plugins")
        .join("bin")
}

fn plugin_name(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let name = file_name.strip_prefix(PLUGIN_PREFIX)?;
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return None;
    }
    Some(name.to_string())
}

fn validate_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(format!("invalid plugin name: {name}"))
    }
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn describe(path: &Path) -> Option<PluginManifest> {
    let mut child = Command::new(path)
        .arg("--describe")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(_) => break,
            None if started.elapsed() > DESCRIBE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_name_accepts_prefix() {
        assert_eq!(
            plugin_name(Path::new("/tmp/xylitol-demo")).as_deref(),
            Some("demo")
        );
        assert_eq!(plugin_name(Path::new("/tmp/demo")), None);
        assert_eq!(plugin_name(Path::new("/tmp/xylitol-")), None);
    }

    #[test]
    fn validate_name_rejects_paths() {
        assert!(validate_name("demo").is_ok());
        assert!(validate_name("../demo").is_err());
        assert!(validate_name("demo/x").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn external_command_name_in_dirs_finds_plugin() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("xylitol-demo");
        let mut file = std::fs::File::create(&path).expect("create");
        file.write_all(b"#!/bin/sh\nexit 0\n").expect("write");
        let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).expect("chmod");

        let args = vec!["xylitol".to_string(), "demo".to_string(), "ui".to_string()];
        assert_eq!(
            external_command_name_in_dirs(&args, &[dir.path().to_path_buf()]).as_deref(),
            Some("demo")
        );
        let unknown = vec!["xylitol".to_string(), "hello".to_string()];
        assert_eq!(
            external_command_name_in_dirs(&unknown, &[dir.path().to_path_buf()]),
            None
        );
        let builtin = vec!["xylitol".to_string(), "tui".to_string()];
        assert_eq!(
            external_command_name_in_dirs(&builtin, &[dir.path().to_path_buf()]),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn discovers_manifest_from_temp_dir() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("xylitol-demo");
        let mut file = std::fs::File::create(&path).expect("create");
        file.write_all(
            b"#!/bin/sh\nprintf '%s' '{\"name\":\"demo\",\"version\":\"0.1.0\",\"requires_host\":true}'\n",
        )
        .expect("write");
        let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).expect("chmod");

        let entries = discover_dirs(&[dir.path().to_path_buf()]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "demo");
        let manifest = entries[0].manifest.as_ref().expect("manifest");
        assert!(manifest.requires_host);
    }
}
