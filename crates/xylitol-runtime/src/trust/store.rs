use std::collections::BTreeMap;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use xylitol_plugin_sdk::error::XyTrustError;

type TrustFile = BTreeMap<String, Option<bool>>;

pub type TrustDecision = Option<bool>;

const CONFIG_DIR_NAME: &str = ".xylitol";
const STORE_VERSION: u32 = 2;
const KEY_FILE_NAME: &str = "trust.key";
const AUDIT_DIR_NAME: &str = "audit";
const AUDIT_FILE_NAME: &str = "trust.log";
const DEFAULT_AUDIT_MAX_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_AUDIT_KEEP_FILES: usize = 5;
const DEFAULT_AUDIT_RETENTION_DAYS: u64 = 90;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustOption {
    pub key: String,
    pub label: String,
    pub trusted: bool,
    pub updates: Vec<TrustUpdate>,
    pub saved_path: Option<String>,
}

pub fn resolve_trust_answer(options: &[TrustOption], answer: Option<&str>) -> Option<usize> {
    let key = answer?;
    options.iter().position(|option| option.key == key)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustUpdate {
    pub path: String,
    pub decision: TrustDecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustActor {
    Tui,
    Host,
    Cli,
    System,
}

impl TrustActor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tui => "tui",
            Self::Host => "host",
            Self::Cli => "cli",
            Self::System => "system",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustSource {
    TuiPrompt,
    HostUnary,
    Resume,
    Startup,
}

impl TrustSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TuiPrompt => "tui_prompt",
            Self::HostUnary => "host_unary",
            Self::Resume => "resume",
            Self::Startup => "startup",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustAuditConfig {
    pub enabled: bool,
    pub max_bytes: u64,
    pub keep_files: usize,
    pub retention_days: u64,
}

impl Default for TrustAuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_bytes: DEFAULT_AUDIT_MAX_BYTES,
            keep_files: DEFAULT_AUDIT_KEEP_FILES,
            retention_days: DEFAULT_AUDIT_RETENTION_DAYS,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TrustAuditContext {
    pub actor: TrustActor,
    pub source: TrustSource,
    pub session_id: Option<String>,
}

impl TrustAuditContext {
    pub fn new(actor: TrustActor, source: TrustSource) -> Self {
        Self {
            actor,
            source,
            session_id: None,
        }
    }

    pub fn system() -> Self {
        Self::new(TrustActor::System, TrustSource::Startup)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionedStore {
    version: u32,
    entries: TrustFile,
    hmac: String,
}

struct LoadedStore {
    entries: TrustFile,
    hmac_ok: bool,
    needs_migration: bool,
}

#[derive(Clone, Debug)]
struct TrustAudit {
    config: TrustAuditConfig,
    log_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TrustManager {
    trust_file_path: PathBuf,
    key_file_path: PathBuf,
    audit: TrustAudit,
}

impl TrustManager {
    pub fn new(trust_dir: impl Into<PathBuf>) -> Self {
        Self::with_audit(trust_dir, TrustAuditConfig::default())
    }

    pub fn with_audit(trust_dir: impl Into<PathBuf>, audit: TrustAuditConfig) -> Self {
        let mut dir = trust_dir.into();
        fs::create_dir_all(&dir).ok();
        let key_file_path = dir.join(KEY_FILE_NAME);
        let log_path = dir.join(AUDIT_DIR_NAME).join(AUDIT_FILE_NAME);
        dir.push("trust.json");
        Self {
            trust_file_path: dir,
            key_file_path,
            audit: TrustAudit {
                config: audit,
                log_path,
            },
        }
    }

    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(CONFIG_DIR_NAME)
    }

    pub fn trust_file_path(&self) -> &Path {
        &self.trust_file_path
    }

    fn normalize_cwd(cwd: &str) -> String {
        match fs::canonicalize(cwd) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => {
                let p = Path::new(cwd);
                if p.is_absolute() {
                    p.to_string_lossy().to_string()
                } else {
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(p)
                        .to_string_lossy()
                        .to_string()
                }
            }
        }
    }

    fn acquire_lock(&self) -> Result<xylitol_plugin_sdk::file_lock::FileLockGuard, XyTrustError> {
        let lock_path = self.trust_file_path.with_extension("json.lock");
        acquire_lock_at(&lock_path)
    }

    fn with_lock<T>(&self, f: impl FnOnce() -> Result<T, XyTrustError>) -> Result<T, XyTrustError> {
        let _guard = self.acquire_lock()?;
        f()
    }

    fn load_store(&self) -> Result<LoadedStore, XyTrustError> {
        if !self.trust_file_path.exists() {
            return Ok(LoadedStore {
                entries: BTreeMap::new(),
                hmac_ok: true,
                needs_migration: false,
            });
        }
        let raw = fs::read_to_string(&self.trust_file_path)
            .map_err(|e| XyTrustError::io("read trust store", e))?;
        let value: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(_) => {
                self.audit_record(
                    "trust_tamper",
                    &TrustAuditContext::system(),
                    None,
                    None,
                    Some(false),
                    Some("invalid_json"),
                );
                return Ok(LoadedStore {
                    entries: BTreeMap::new(),
                    hmac_ok: false,
                    needs_migration: false,
                });
            }
        };
        let permissions_loose = !has_private_permissions(&self.trust_file_path);
        if value.get("version").is_some() {
            let store: VersionedStore = match serde_json::from_value(value) {
                Ok(store) => store,
                Err(_) => {
                    self.audit_record(
                        "trust_tamper",
                        &TrustAuditContext::system(),
                        None,
                        None,
                        Some(false),
                        Some("invalid_schema"),
                    );
                    return Ok(LoadedStore {
                        entries: BTreeMap::new(),
                        hmac_ok: false,
                        needs_migration: false,
                    });
                }
            };
            let Some(key) = self.read_key() else {
                self.audit_record(
                    "trust_tamper",
                    &TrustAuditContext::system(),
                    None,
                    None,
                    Some(false),
                    Some("missing_key"),
                );
                return Ok(LoadedStore {
                    entries: BTreeMap::new(),
                    hmac_ok: false,
                    needs_migration: false,
                });
            };
            let expected = hmac_hex(&key, store.version, &store.entries);
            if !constant_time_eq(expected.as_bytes(), store.hmac.as_bytes()) {
                self.audit_record(
                    "trust_tamper",
                    &TrustAuditContext::system(),
                    None,
                    None,
                    Some(false),
                    Some("hmac_mismatch"),
                );
                return Ok(LoadedStore {
                    entries: BTreeMap::new(),
                    hmac_ok: false,
                    needs_migration: false,
                });
            }
            return Ok(LoadedStore {
                entries: store.entries,
                hmac_ok: true,
                needs_migration: permissions_loose,
            });
        }
        let entries: TrustFile = match serde_json::from_value(value) {
            Ok(entries) => entries,
            Err(_) => {
                self.audit_record(
                    "trust_tamper",
                    &TrustAuditContext::system(),
                    None,
                    None,
                    Some(false),
                    Some("invalid_legacy_schema"),
                );
                return Ok(LoadedStore {
                    entries: BTreeMap::new(),
                    hmac_ok: false,
                    needs_migration: false,
                });
            }
        };
        Ok(LoadedStore {
            entries,
            hmac_ok: true,
            needs_migration: true,
        })
    }

    fn load_and_migrate(&self) -> Result<LoadedStore, XyTrustError> {
        let loaded = self.load_store()?;
        if loaded.needs_migration && loaded.hmac_ok {
            let reason = if loaded.entries.is_empty() {
                "migrate_permissions"
            } else {
                "migrate_schema"
            };
            match self.write_store(&loaded.entries, &TrustAuditContext::system(), reason) {
                Ok(()) => {}
                Err(e) => log::warn!(target: "xylitol::trust", "trust store migration failed: {e}"),
            }
        }
        Ok(loaded)
    }

    fn read_key(&self) -> Option<Vec<u8>> {
        let raw = fs::read_to_string(&self.key_file_path).ok()?;
        let decoded = hex_decode(raw.trim())?;
        if decoded.len() == 32 {
            Some(decoded)
        } else {
            None
        }
    }

    fn ensure_key(&self) -> Result<Vec<u8>, XyTrustError> {
        if let Some(key) = self.read_key() {
            set_private_permissions(&self.key_file_path)?;
            return Ok(key);
        }
        let key = generate_key();
        write_atomic_private(&self.key_file_path, &hex_encode(&key))?;
        Ok(key)
    }

    fn write_store(
        &self,
        data: &TrustFile,
        ctx: &TrustAuditContext,
        reason: &str,
    ) -> Result<(), XyTrustError> {
        if let Some(parent) = self.trust_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| XyTrustError::io("create trust dir", e))?;
        }
        let key = self.ensure_key()?;
        let hmac = hmac_hex(&key, STORE_VERSION, data);
        let store = VersionedStore {
            version: STORE_VERSION,
            entries: data.clone(),
            hmac,
        };
        let json = serde_json::to_string_pretty(&store)?;
        write_atomic_private(&self.trust_file_path, &format!("{json}\n"))?;
        self.audit_record(
            "trust_store_write",
            ctx,
            None,
            None,
            Some(true),
            Some(reason),
        );
        Ok(())
    }

    pub fn is_project_trusted(&self, cwd: &str) -> TrustDecision {
        self.with_lock(|| {
            let loaded = self.load_and_migrate()?;
            Ok(find_nearest_decision(&loaded.entries, cwd))
        })
        .unwrap_or(None)
    }

    pub fn is_trusted(&self, cwd: &str) -> bool {
        self.is_project_trusted(cwd) == Some(true)
    }

    pub fn recheck(&self, cwd: &str, ctx: &TrustAuditContext) -> TrustDecision {
        self.with_lock(|| {
            let loaded = self.load_and_migrate()?;
            let decision = find_nearest_decision(&loaded.entries, cwd);
            self.audit_record(
                "trust_recheck",
                ctx,
                Some(&Self::normalize_cwd(cwd)),
                decision,
                Some(loaded.hmac_ok),
                Some("resume"),
            );
            Ok(decision)
        })
        .unwrap_or(None)
    }

    pub fn record_decision(
        &self,
        event: &str,
        cwd: &str,
        decision: TrustDecision,
        ctx: &TrustAuditContext,
        reason: &str,
    ) {
        self.audit_record(
            event,
            ctx,
            Some(&Self::normalize_cwd(cwd)),
            decision,
            None,
            Some(reason),
        );
    }

    pub fn apply_updates(
        &self,
        updates: &[TrustUpdate],
        ctx: &TrustAuditContext,
    ) -> Result<(), XyTrustError> {
        self.with_lock(|| {
            let loaded = self.load_and_migrate()?;
            let mut data = loaded.entries;
            for update in updates {
                let key = Self::normalize_cwd(&update.path);
                match update.decision {
                    Some(b) => {
                        data.insert(key, Some(b));
                    }
                    None => {
                        data.remove(&key);
                    }
                }
            }
            self.write_store(&data, ctx, "apply_updates")?;
            for update in updates {
                let event = match update.decision {
                    Some(true) => "trust_grant",
                    Some(false) => "trust_deny",
                    None => "trust_revoke",
                };
                self.audit_record(
                    event,
                    ctx,
                    Some(&Self::normalize_cwd(&update.path)),
                    update.decision,
                    Some(true),
                    Some("apply_updates"),
                );
            }
            Ok(())
        })
    }

    pub fn get_trust_options(&self, cwd: &str, include_session_only: bool) -> Vec<TrustOption> {
        let trust_path = Self::normalize_cwd(cwd);
        let parent_path = parent_dir(&trust_path);

        let mut options = vec![TrustOption {
            key: "trust".into(),
            label: "Trust".into(),
            trusted: true,
            updates: vec![TrustUpdate {
                path: trust_path.clone(),
                decision: Some(true),
            }],
            saved_path: Some(trust_path.clone()),
        }];

        if let Some(ref parent) = parent_path
            && *parent != trust_path
        {
            options.push(TrustOption {
                key: "trust_parent".into(),
                label: format!("Trust parent folder ({parent})"),
                trusted: true,
                updates: vec![
                    TrustUpdate {
                        path: parent.clone(),
                        decision: Some(true),
                    },
                    TrustUpdate {
                        path: trust_path.clone(),
                        decision: None,
                    },
                ],
                saved_path: Some(parent.clone()),
            });
        }

        if include_session_only {
            options.push(TrustOption {
                key: "trust_session".into(),
                label: "Trust (this session only)".into(),
                trusted: true,
                updates: vec![],
                saved_path: None,
            });
        }

        options.push(TrustOption {
            key: "deny".into(),
            label: "Do not trust".into(),
            trusted: false,
            updates: vec![TrustUpdate {
                path: trust_path.clone(),
                decision: Some(false),
            }],
            saved_path: Some(trust_path),
        });

        if include_session_only {
            options.push(TrustOption {
                key: "deny_session".into(),
                label: "Do not trust (this session only)".into(),
                trusted: false,
                updates: vec![],
                saved_path: None,
            });
        }

        options
    }

    pub fn has_trust_inputs(&self, cwd: &str) -> bool {
        let mut current = match fs::canonicalize(cwd) {
            Ok(p) => p,
            Err(_) => return false,
        };
        if current.join(CONFIG_DIR_NAME).is_dir() {
            return true;
        }
        loop {
            if current.join(".agents").join("skills").is_dir() {
                return true;
            }
            match current.parent() {
                Some(parent) if parent != current => current = parent.to_path_buf(),
                _ => break,
            }
        }
        false
    }

    fn audit_record(
        &self,
        event: &str,
        ctx: &TrustAuditContext,
        workspace: Option<&str>,
        decision: TrustDecision,
        hmac_ok: Option<bool>,
        reason: Option<&str>,
    ) {
        if !self.audit.config.enabled {
            return;
        }
        let ts = OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_default();
        let record = serde_json::json!({
            "ts": ts,
            "event": event,
            "actor": ctx.actor.as_str(),
            "session_id": ctx.session_id,
            "workspace": workspace,
            "decision": decision,
            "source": ctx.source.as_str(),
            "hmac_ok": hmac_ok,
            "reason": reason,
        });
        let Ok(line) = serde_json::to_string(&record) else {
            return;
        };
        if let Err(e) = self.append_audit_line(&line) {
            log::warn!(target: "xylitol::trust", "trust audit write failed: {e}");
        }
    }

    fn append_audit_line(&self, line: &str) -> std::io::Result<()> {
        if let Some(parent) = self.audit.log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let incoming = (line.len() + 1) as u64;
        self.rotate_audit_if_needed(incoming)?;
        let mut file = open_private_append(&self.audit.log_path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        set_private_permissions_io(&self.audit.log_path)?;
        Ok(())
    }

    fn rotate_audit_if_needed(&self, incoming: u64) -> std::io::Result<()> {
        let size = fs::metadata(&self.audit.log_path)
            .map(|m| m.len())
            .unwrap_or(0);
        if size + incoming <= self.audit.config.max_bytes {
            return Ok(());
        }
        let keep = self.audit.config.keep_files.max(1);
        if keep > 1 {
            for index in (1..keep - 1).rev() {
                let from = audit_archive_path(&self.audit.log_path, index);
                let to = audit_archive_path(&self.audit.log_path, index + 1);
                if from.exists() {
                    let _ = fs::rename(&from, &to);
                }
            }
            let first = audit_archive_path(&self.audit.log_path, 1);
            if self.audit.log_path.exists() {
                let _ = fs::rename(&self.audit.log_path, &first);
            }
        } else {
            let _ = fs::remove_file(&self.audit.log_path);
        }
        self.prune_audit_archives(keep);
        Ok(())
    }

    fn prune_audit_archives(&self, keep: usize) {
        let cutoff = SystemTime::now()
            .checked_sub(Duration::from_secs(
                self.audit.config.retention_days.saturating_mul(86_400),
            ))
            .unwrap_or(SystemTime::UNIX_EPOCH);
        for index in 1..keep {
            let path = audit_archive_path(&self.audit.log_path, index);
            let Ok(meta) = fs::metadata(&path) else {
                continue;
            };
            if let Ok(modified) = meta.modified()
                && modified < cutoff
            {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

fn audit_archive_path(base: &Path, index: usize) -> PathBuf {
    base.with_file_name(format!("{AUDIT_FILE_NAME}.{index}"))
}

fn generate_key() -> Vec<u8> {
    let mut key = Vec::with_capacity(32);
    key.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    key.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    key
}

fn hmac_hex(key: &[u8], version: u32, entries: &TrustFile) -> String {
    let body = format!(
        "{version}\n{}",
        serde_json::to_string(entries).unwrap_or_default()
    );
    hex_encode(&hmac_sha256(key, body.as_bytes()))
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut normalized = [0u8; BLOCK];
    if key.len() > BLOCK {
        normalized[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0x36u8; BLOCK];
    let mut outer_pad = [0x5cu8; BLOCK];
    for index in 0..BLOCK {
        inner_pad[index] ^= normalized[index];
        outer_pad[index] ^= normalized[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer.finalize().into()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn hex_decode(raw: &str) -> Option<Vec<u8>> {
    if !raw.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(raw.len() / 2);
    let bytes = raw.as_bytes();
    for pair in bytes.as_chunks::<2>().0 {
        let high = (pair[0] as char).to_digit(16)?;
        let low = (pair[1] as char).to_digit(16)?;
        out.push((high * 16 + low) as u8);
    }
    Some(out)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

fn find_nearest_decision(data: &TrustFile, cwd: &str) -> TrustDecision {
    let mut current = TrustManager::normalize_cwd(cwd);
    loop {
        if let Some(decision) = data.get(&current) {
            return *decision;
        }
        let parent = parent_dir(&current)?;
        if parent == current {
            return None;
        }
        current = parent;
    }
}

fn parent_dir(path_str: &str) -> Option<String> {
    let p = Path::new(path_str);
    let parent = p.parent()?;
    if parent == p {
        return None;
    }
    Some(parent.to_string_lossy().to_string())
}

fn has_private_permissions(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o777 == 0o600)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        true
    }
}

fn set_private_permissions(path: &Path) -> Result<(), XyTrustError> {
    set_private_permissions_io(path).map_err(|e| XyTrustError::io("set trust permissions", e))
}

fn set_private_permissions_io(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

fn open_private_append(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn write_atomic_private(path: &Path, contents: &str) -> Result<(), XyTrustError> {
    let tmp = path.with_file_name(format!(
        "{}.tmp",
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "trust.tmp".into())
    ));
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&tmp)
        .map_err(|e| XyTrustError::io("write trust tmp", e))?;
    file.write_all(contents.as_bytes())
        .map_err(|e| XyTrustError::io("write trust tmp", e))?;
    file.sync_all()
        .map_err(|e| XyTrustError::io("sync trust tmp", e))?;
    drop(file);
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(path);
        fs::rename(&tmp, path).map_err(|e2| XyTrustError::io("rename trust store", e2))?;
        log::warn!(target: "xylitol::trust", "atomic rename fallback after: {e}");
    }
    set_private_permissions(path)?;
    Ok(())
}

fn acquire_lock_at(
    lock_path: &Path,
) -> Result<xylitol_plugin_sdk::file_lock::FileLockGuard, XyTrustError> {
    acquire_lock_at_with_stale(
        lock_path,
        xylitol_plugin_sdk::file_lock::DEFAULT_STALE_AFTER,
    )
}

fn acquire_lock_at_with_stale(
    lock_path: &Path,
    stale_after: Duration,
) -> Result<xylitol_plugin_sdk::file_lock::FileLockGuard, XyTrustError> {
    xylitol_plugin_sdk::file_lock::FileLockGuard::acquire_with_stale(
        lock_path.to_path_buf(),
        stale_after,
    )
    .map_err(|e| {
        if e.kind() == ErrorKind::AlreadyExists {
            XyTrustError::Lock
        } else {
            XyTrustError::io("create trust lock", e)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (TrustManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let mgr = TrustManager::new(dir.path().to_path_buf());
        (mgr, dir)
    }

    fn ctx() -> TrustAuditContext {
        TrustAuditContext::new(TrustActor::Cli, TrustSource::Startup)
    }

    #[test]
    fn test_default_is_undecided() {
        let (mgr, _dir) = setup();
        assert_eq!(mgr.is_project_trusted("/some/project"), None);
    }

    #[test]
    fn test_set_and_check_trust() {
        let (mgr, dir) = setup();
        let path = dir.path().join("my-project");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.apply_updates(
            &[TrustUpdate {
                path: path_str.clone(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), Some(true));
    }

    #[test]
    fn test_set_untrusted() {
        let (mgr, dir) = setup();
        let path = dir.path().join("untrusted-project");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.apply_updates(
            &[TrustUpdate {
                path: path_str.clone(),
                decision: Some(false),
            }],
            &ctx(),
        )
        .unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), Some(false));
    }

    #[test]
    fn test_parent_inheritance() {
        let (mgr, dir) = setup();
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        mgr.apply_updates(
            &[TrustUpdate {
                path: parent.to_string_lossy().to_string(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();
        assert_eq!(mgr.is_project_trusted(&child.to_string_lossy()), Some(true));
    }

    #[test]
    fn test_child_override_nearest_ancestor_wins() {
        let (mgr, dir) = setup();
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        mgr.apply_updates(
            &[TrustUpdate {
                path: parent.to_string_lossy().to_string(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();
        mgr.apply_updates(
            &[TrustUpdate {
                path: child.to_string_lossy().to_string(),
                decision: Some(false),
            }],
            &ctx(),
        )
        .unwrap();
        assert_eq!(
            mgr.is_project_trusted(&child.to_string_lossy()),
            Some(false)
        );
        let sibling = parent.join("sibling");
        fs::create_dir_all(&sibling).unwrap();
        assert_eq!(
            mgr.is_project_trusted(&sibling.to_string_lossy()),
            Some(true)
        );
    }

    #[test]
    fn test_clear_trust() {
        let (mgr, dir) = setup();
        let path = dir.path().join("cleared");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();

        mgr.apply_updates(
            &[TrustUpdate {
                path: path_str.clone(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();
        mgr.apply_updates(
            &[TrustUpdate {
                path: path_str.clone(),
                decision: None,
            }],
            &ctx(),
        )
        .unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), None);
    }

    #[test]
    fn test_apply_updates_multiple() {
        let (mgr, dir) = setup();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let a_str = a.to_string_lossy().to_string();
        let b_str = b.to_string_lossy().to_string();

        mgr.apply_updates(
            &[
                TrustUpdate {
                    path: a_str.clone(),
                    decision: Some(true),
                },
                TrustUpdate {
                    path: b_str.clone(),
                    decision: Some(false),
                },
            ],
            &ctx(),
        )
        .unwrap();
        assert_eq!(mgr.is_project_trusted(&a_str), Some(true));
        assert_eq!(mgr.is_project_trusted(&b_str), Some(false));
    }

    #[test]
    fn test_get_trust_options() {
        let (mgr, dir) = setup();
        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();
        let cwd_str = cwd.to_string_lossy().to_string();

        let options = mgr.get_trust_options(&cwd_str, false);
        assert!(options.len() >= 2);
        assert!(options.iter().any(|o| o.label == "Trust"));
        assert!(options.iter().any(|o| o.label.contains("Do not trust")));
    }

    #[test]
    fn test_get_trust_options_with_session() {
        let (mgr, dir) = setup();
        let cwd = dir.path().join("project");
        fs::create_dir_all(&cwd).unwrap();
        let cwd_str = cwd.to_string_lossy().to_string();

        let options = mgr.get_trust_options(&cwd_str, true);
        let session_only = options
            .iter()
            .filter(|o| o.label.contains("this session only"))
            .count();
        assert_eq!(session_only, 2, "expected two session-only variants");
        assert_eq!(
            mgr.get_trust_options(&cwd_str, false).len() + 2,
            options.len()
        );
    }

    #[test]
    fn test_has_trust_inputs_no_inputs() {
        let (mgr, dir) = setup();
        assert!(!mgr.has_trust_inputs(&dir.path().to_string_lossy()));
    }

    #[test]
    fn test_has_trust_inputs_with_config_dir() {
        let (mgr, dir) = setup();
        fs::create_dir_all(dir.path().join(".xylitol")).unwrap();
        assert!(mgr.has_trust_inputs(&dir.path().to_string_lossy()));
    }

    #[test]
    fn test_has_trust_inputs_with_skills_ancestor() {
        let (mgr, dir) = setup();
        let child = dir.path().join("deep").join("project");
        fs::create_dir_all(&child).unwrap();
        fs::create_dir_all(dir.path().join(".agents").join("skills")).unwrap();
        assert!(mgr.has_trust_inputs(&child.to_string_lossy()));
    }

    #[test]
    fn test_acquire_lock_succeeds_and_releases() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join("trust.json.lock");
        let guard = acquire_lock_at(&lock_path).expect("first acquire");
        assert!(lock_path.exists());
        drop(guard);
        assert!(!lock_path.exists(), "RAII guard MUST remove the lock file");
        acquire_lock_at(&lock_path).expect("re-acquire after release");
    }

    #[test]
    fn test_acquire_lock_reclaims_stale_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join("trust.json.lock");
        fs::write(&lock_path, "stale").expect("write stale lock");
        let guard = acquire_lock_at_with_stale(&lock_path, Duration::ZERO)
            .expect("stale lock MUST be reclaimed");
        drop(guard);
        assert!(!lock_path.exists());
    }

    #[test]
    fn test_lock_guard_removes_file_on_panic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock_path = dir.path().join("trust.json.lock");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = acquire_lock_at(&lock_path).expect("acquire");
            panic!("boom");
        }));
        assert!(result.is_err());
        assert!(!lock_path.exists(), "panic MUST NOT leave a dead lock file");
    }

    #[test]
    fn test_persistence_format_is_versioned() {
        let (mgr, dir) = setup();
        let path = dir.path().join("proj");
        fs::create_dir_all(&path).unwrap();
        mgr.apply_updates(
            &[TrustUpdate {
                path: path.to_string_lossy().to_string(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();

        let on_disk = fs::read_to_string(mgr.trust_file_path()).expect("read trust file");
        let value: serde_json::Value = serde_json::from_str(&on_disk).expect("json");
        assert_eq!(value["version"], STORE_VERSION);
        assert!(value["hmac"].as_str().is_some_and(|s| s.len() == 64));
        assert!(value["entries"].is_object());
    }

    #[cfg(unix)]
    #[test]
    fn test_store_key_and_lock_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let (mgr, dir) = setup();
        let path = dir.path().join("proj");
        fs::create_dir_all(&path).unwrap();
        mgr.apply_updates(
            &[TrustUpdate {
                path: path.to_string_lossy().to_string(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();
        for file in [
            mgr.trust_file_path().to_path_buf(),
            dir.path().join("trust.key"),
        ] {
            let mode = fs::metadata(&file).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{} must be 0600", file.display());
        }
        let lock_path = mgr.trust_file_path().with_extension("json.lock");
        let _lock = mgr.acquire_lock().expect("lock");
        let mode = fs::metadata(&lock_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "lock must be 0600");
    }

    #[test]
    fn test_legacy_store_migrates_to_versioned_format() {
        let (mgr, dir) = setup();
        let path = dir.path().join("legacy");
        fs::create_dir_all(&path).unwrap();
        let key = TrustManager::normalize_cwd(&path.to_string_lossy());
        let legacy = serde_json::json!({ key: true });
        fs::write(
            mgr.trust_file_path(),
            serde_json::to_string_pretty(&legacy).unwrap(),
        )
        .unwrap();

        assert_eq!(mgr.is_project_trusted(&path.to_string_lossy()), Some(true));
        let on_disk = fs::read_to_string(mgr.trust_file_path()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&on_disk).unwrap();
        assert_eq!(value["version"], STORE_VERSION);
        assert!(dir.path().join("trust.key").exists());
    }

    #[test]
    fn test_tampered_store_is_rejected() {
        let (mgr, dir) = setup();
        let path = dir.path().join("tampered");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();
        mgr.apply_updates(
            &[TrustUpdate {
                path: path_str.clone(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();

        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(mgr.trust_file_path()).unwrap()).unwrap();
        value["entries"][TrustManager::normalize_cwd(&path_str)] = serde_json::json!(false);
        fs::write(
            mgr.trust_file_path(),
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();

        assert_eq!(mgr.is_project_trusted(&path_str), None);
        let log = fs::read_to_string(dir.path().join("audit/trust.log")).unwrap();
        assert!(
            log.contains("trust_tamper"),
            "audit must record tamper: {log}"
        );
    }

    #[test]
    fn test_missing_key_is_tamper() {
        let (mgr, dir) = setup();
        let path = dir.path().join("nokey");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();
        mgr.apply_updates(
            &[TrustUpdate {
                path: path_str.clone(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();
        fs::remove_file(dir.path().join("trust.key")).unwrap();
        assert_eq!(mgr.is_project_trusted(&path_str), None);
    }

    #[test]
    fn test_audit_records_grant_and_store_write() {
        let (mgr, dir) = setup();
        let path = dir.path().join("audited");
        fs::create_dir_all(&path).unwrap();
        mgr.apply_updates(
            &[TrustUpdate {
                path: path.to_string_lossy().to_string(),
                decision: Some(true),
            }],
            &TrustAuditContext::new(TrustActor::Tui, TrustSource::TuiPrompt),
        )
        .unwrap();

        let log = fs::read_to_string(dir.path().join("audit/trust.log")).unwrap();
        let lines: Vec<serde_json::Value> = log
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(lines.iter().any(|v| v["event"] == "trust_store_write"));
        let grant = lines
            .iter()
            .find(|v| v["event"] == "trust_grant")
            .expect("grant event");
        assert_eq!(grant["actor"], "tui");
        assert_eq!(grant["source"], "tui_prompt");
        assert_eq!(grant["decision"], true);
        assert_eq!(grant["hmac_ok"], true);
        assert!(grant["ts"].as_str().is_some_and(|s| s.contains('T')));
    }

    #[test]
    fn test_audit_disabled_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = TrustManager::with_audit(
            dir.path().to_path_buf(),
            TrustAuditConfig {
                enabled: false,
                ..Default::default()
            },
        );
        let path = dir.path().join("quiet");
        fs::create_dir_all(&path).unwrap();
        mgr.apply_updates(
            &[TrustUpdate {
                path: path.to_string_lossy().to_string(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();
        assert!(!dir.path().join("audit/trust.log").exists());
    }

    #[test]
    fn test_audit_rotates_when_full() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = TrustManager::with_audit(
            dir.path().to_path_buf(),
            TrustAuditConfig {
                enabled: true,
                max_bytes: 200,
                keep_files: 3,
                retention_days: 90,
            },
        );
        for index in 0..10 {
            mgr.audit_record(
                "trust_recheck",
                &ctx(),
                Some("/tmp/workspace"),
                Some(true),
                Some(true),
                Some(&format!("round-{index}")),
            );
        }
        assert!(dir.path().join("audit/trust.log").exists());
        assert!(dir.path().join("audit/trust.log.1").exists());
    }

    #[test]
    fn test_recheck_audits_resume_source() {
        let (mgr, dir) = setup();
        let path = dir.path().join("resume");
        fs::create_dir_all(&path).unwrap();
        let path_str = path.to_string_lossy().to_string();
        mgr.apply_updates(
            &[TrustUpdate {
                path: path_str.clone(),
                decision: Some(true),
            }],
            &ctx(),
        )
        .unwrap();
        let decision = mgr.recheck(
            &path_str,
            &TrustAuditContext::new(TrustActor::Host, TrustSource::Resume),
        );
        assert_eq!(decision, Some(true));
        let log = fs::read_to_string(dir.path().join("audit/trust.log")).unwrap();
        let recheck = log
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .find(|v| v["event"] == "trust_recheck")
            .expect("recheck event");
        assert_eq!(recheck["source"], "resume");
        assert_eq!(recheck["actor"], "host");
        assert_eq!(recheck["hmac_ok"], true);
    }
}
