use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub shell: PathBuf,
    pub args: Vec<String>,
}

pub fn find_bash(custom_shell: Option<&std::path::Path>) -> ShellConfig {
    if let Some(path) = custom_shell
        && path.exists()
    {
        return bash_config(path);
    }

    if cfg!(target_os = "windows") {
        find_windows_bash()
    } else {
        find_unix_bash()
    }
}

fn bash_config(path: &std::path::Path) -> ShellConfig {
    let is_legacy_wsl = cfg!(target_os = "windows")
        && path
            .to_string_lossy()
            .to_lowercase()
            .contains("windows\\system32");

    let args = if is_legacy_wsl {
        vec!["-s".to_string()]
    } else {
        vec!["-c".to_string()]
    };

    ShellConfig {
        shell: path.to_path_buf(),
        args,
    }
}

#[cfg(target_os = "windows")]
fn find_windows_bash() -> ShellConfig {
    let candidates = [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ];

    for candidate in &candidates {
        let path = std::path::Path::new(candidate);
        if path.exists() {
            return bash_config(path);
        }
    }

    if let Some(path) = which("bash.exe") {
        return bash_config(&path);
    }

    bash_config(std::path::Path::new("bash.exe"))
}

#[cfg(not(target_os = "windows"))]
fn find_windows_bash() -> ShellConfig {
    unreachable!("not Windows")
}

#[cfg(not(target_os = "windows"))]
fn find_unix_bash() -> ShellConfig {
    let bash_path = std::path::Path::new("/bin/bash");
    if bash_path.exists() {
        return bash_config(bash_path);
    }

    if let Some(path) = which("bash") {
        return bash_config(&path);
    }

    bash_config(std::path::Path::new("sh"))
}

#[cfg(target_os = "windows")]
fn find_unix_bash() -> ShellConfig {
    unreachable!("not Unix")
}

fn which(name: &str) -> Option<PathBuf> {
    let output = Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    None
}

pub fn current_exe_bin_dir() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(ToOwned::to_owned)
}

pub fn shell_env_with_agent_bin(
    mut base: std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    let Some(bin) = current_exe_bin_dir() else {
        return base;
    };
    let existing = base.get("PATH").cloned().unwrap_or_default();
    let mut list: Vec<std::path::PathBuf> = std::env::split_paths(&existing).collect();
    list.insert(0, bin.clone());
    if let Ok(joined) = std::env::join_paths(&list) {
        base.insert("PATH".into(), joined.to_string_lossy().into_owned());
    }
    base
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn test_find_bash_unix_finds_bash_or_sh() {
        if cfg!(not(target_os = "windows")) {
            let config = find_bash(None);
            let name = config.shell.file_name().unwrap().to_string_lossy();
            assert!(
                name == "bash" || name == "sh",
                "expected bash or sh, got {name}"
            );
            assert_eq!(config.args, vec!["-c"]);
        }
    }

    #[test]
    fn test_shell_env_prepends_agent_bin() {
        let mut base = BTreeMap::new();
        base.insert("PATH".into(), "/usr/bin:/bin".into());
        let env = shell_env_with_agent_bin(base);
        let path = env.get("PATH").unwrap();
        let mut list = std::env::split_paths(path);
        let first = list.next().expect("PATH 非空");
        let bin = current_exe_bin_dir().expect("current exe dir");
        assert_eq!(first, bin, "agent bin MUST 前置");
    }

    #[test]
    fn test_shell_env_without_path_creates_one() {
        let base = BTreeMap::new();
        let env = shell_env_with_agent_bin(base);
        let path = env.get("PATH").expect("无 PATH 时 MUST 创建");
        let mut list = std::env::split_paths(path);
        let first = list.next().expect("PATH 非空");
        let bin = current_exe_bin_dir().expect("current exe dir");
        assert_eq!(first, bin);
    }
}
