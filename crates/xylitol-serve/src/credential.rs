use std::io::Write;
use std::path::{Path, PathBuf};

use base64::Engine as _;

pub const CREDENTIAL_FILE_NAME: &str = "serve.token";

pub fn default_credential_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xylitol")
        .join(CREDENTIAL_FILE_NAME)
}

pub fn load_or_create_credential(path: &Path) -> std::io::Result<String> {
    match std::fs::read_to_string(path) {
        Ok(raw) => {
            let token = raw.trim();
            if !token.is_empty() {
                ensure_private(path)?;
                return Ok(token.to_string());
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    let token = generate_token();
    write_private_file(path, &token)?;
    Ok(token)
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn write_private_file(path: &Path, body: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("tmp");
    let tmp = path.with_extension(format!("{ext}.tmp"));
    let _ = std::fs::remove_file(&tmp);
    {
        let mut file = {
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                std::fs::OpenOptions::new()
                    .mode(0o600)
                    .write(true)
                    .create_new(true)
                    .open(&tmp)?
            }
            #[cfg(not(unix))]
            {
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&tmp)?
            }
        };
        file.write_all(body.as_bytes())?;
        file.flush()?;
    }
    std::fs::rename(&tmp, path)
}

fn ensure_private(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!("xylitol-cred-{}", uuid::Uuid::new_v4()))
            .join(CREDENTIAL_FILE_NAME)
    }

    #[test]
    fn creates_stable_0600_credential() {
        let path = temp_path();
        let first = load_or_create_credential(&path).expect("create");
        assert!(first.len() >= 32, "generated token must be strong");
        assert!(!path.with_extension("token.tmp").exists(), "tmp renamed");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "credential must be 0600");
        }

        let second = load_or_create_credential(&path).expect("reuse");
        assert_eq!(first, second, "credential must be stable across restarts");
        let _ = std::fs::remove_dir_all(path.parent().expect("parent"));
    }

    #[test]
    fn empty_file_regenerates() {
        let path = temp_path();
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(&path, "\n").expect("write empty");
        let token = load_or_create_credential(&path).expect("regenerate");
        assert!(!token.trim().is_empty());
        let _ = std::fs::remove_dir_all(path.parent().expect("parent"));
    }

    #[test]
    fn generated_tokens_differ() {
        assert_ne!(generate_token(), generate_token());
    }
}
