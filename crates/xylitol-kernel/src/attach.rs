use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use thiserror::Error;

pub const DEFAULT_ATTACH_URL: &str = "http://127.0.0.1:18790";

#[derive(Debug, Error)]
pub enum AttachError {
    #[error("invalid attach URL {url}: {source}")]
    InvalidUrl {
        url: String,
        source: url::ParseError,
    },
    #[error("Host is not listening at {url}")]
    NotListening { url: String },
}

pub fn probe_host(url: &str) -> Result<(), AttachError> {
    let parsed = url::Url::parse(url).map_err(|source| AttachError::InvalidUrl {
        url: url.to_string(),
        source,
    })?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port = parsed.port_or_known_default().unwrap_or(18790);
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|_| AttachError::NotListening {
            url: url.to_string(),
        })?;
    for addr in addrs {
        if tcp_connect(addr).is_ok() {
            return Ok(());
        }
    }
    Err(AttachError::NotListening {
        url: url.to_string(),
    })
}

fn tcp_connect(addr: SocketAddr) -> std::io::Result<TcpStream> {
    TcpStream::connect_timeout(&addr, Duration::from_millis(400))
}

pub fn resolve_attach_url(attach: Option<&str>, port: Option<u16>) -> String {
    if let Some(url) = attach.map(str::trim).filter(|s| !s.is_empty()) {
        return url.to_string();
    }
    match port {
        Some(p) => format!("http://127.0.0.1:{p}"),
        None => DEFAULT_ATTACH_URL.to_string(),
    }
}

pub fn attach_fail_message(url: &str) -> String {
    format!(
        "Host is not listening at {url}. Start it with: xylitol serve\n\
         (default http://127.0.0.1:18790)"
    )
}

pub fn spawn_failed_message(url: &str, err: &str) -> String {
    format!(
        "Failed to start Host at {url}: {err}. \
         Confirm `{exe} serve` can bind that address, or run: xylitol serve",
        exe = std::env::current_exe()
            .ok()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "xylitol".into())
    )
}

pub fn ready_timeout_message(url: &str) -> String {
    format!(
        "Timed out waiting for Host at {url} to become ready \
         (/healthz 200). Inspect the serve process logs, or run: xylitol serve"
    )
}

pub fn should_ensure_host(attach: Option<&str>, port: Option<u16>, no_ensure: bool) -> bool {
    if no_ensure {
        return false;
    }
    let url = resolve_attach_url(attach, port);
    let implied = resolve_attach_url(None, port);
    same_url(&url, &implied) && is_loopback_url(&url)
}

fn is_loopback_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    matches!(normalize_host(parsed.host_str()), "127.0.0.1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_fails_when_nothing_listens() {
        let err = probe_host("http://127.0.0.1:1").unwrap_err();
        assert!(matches!(err, AttachError::NotListening { .. }));
    }

    #[test]
    fn resolve_attach_url_prefers_attach_over_port() {
        assert_eq!(DEFAULT_ATTACH_URL, "http://127.0.0.1:18790");
        assert_eq!(resolve_attach_url(None, None), DEFAULT_ATTACH_URL);
        assert_eq!(resolve_attach_url(None, Some(9)), "http://127.0.0.1:9");
        assert_eq!(
            resolve_attach_url(Some("http://127.0.0.1:77"), Some(9)),
            "http://127.0.0.1:77"
        );
        assert_eq!(
            resolve_attach_url(Some("  "), Some(9)),
            "http://127.0.0.1:9"
        );
    }

    #[test]
    fn should_ensure_host_only_loopback_implied_url() {
        assert!(should_ensure_host(None, None, false));
        assert!(should_ensure_host(None, Some(9), false));
        assert!(should_ensure_host(
            Some("http://127.0.0.1:18790"),
            None,
            false
        ));
        assert!(should_ensure_host(
            Some("http://localhost:18790"),
            None,
            false
        ));
        assert!(!should_ensure_host(
            Some("http://127.0.0.1:9"),
            Some(11),
            false
        ));
        assert!(!should_ensure_host(
            Some("http://example.com:18790"),
            None,
            false
        ));
        assert!(!should_ensure_host(None, None, true));
    }

    #[test]
    fn ensure_diagnostics_are_actionable() {
        let spawn = spawn_failed_message("http://127.0.0.1:18790", "boom");
        let timeout = ready_timeout_message("http://127.0.0.1:18790");
        assert!(spawn.contains("Failed to start Host"), "{spawn}");
        assert!(spawn.contains("bind"), "{spawn}");
        assert!(timeout.contains("Timed out"), "{timeout}");
        assert!(timeout.contains("/healthz"), "{timeout}");
    }
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthzInfo {
    pub pid: u32,
    pub version: String,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionVerdict {
    Same,
    Older,
    Newer,
    Unparsable,
}

#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadyDecision {
    Attach,
    Replace,
    FailClosed,
}

#[cfg(feature = "server")]
pub fn version_verdict(own: &str, peer: &str) -> VersionVerdict {
    let (Ok(own), Ok(peer)) = (semver::Version::parse(own), semver::Version::parse(peer)) else {
        return VersionVerdict::Unparsable;
    };
    match peer.cmp(&own) {
        std::cmp::Ordering::Equal => VersionVerdict::Same,
        std::cmp::Ordering::Less => VersionVerdict::Older,
        std::cmp::Ordering::Greater => VersionVerdict::Newer,
    }
}

#[cfg(feature = "server")]
pub fn ready_decision(hz: Option<&HealthzInfo>, own: &str) -> ReadyDecision {
    let Some(hz) = hz else {
        return ReadyDecision::FailClosed;
    };
    match version_verdict(own, &hz.version) {
        VersionVerdict::Same => ReadyDecision::Attach,
        VersionVerdict::Older => ReadyDecision::Replace,
        VersionVerdict::Newer | VersionVerdict::Unparsable => ReadyDecision::FailClosed,
    }
}

#[cfg(feature = "server")]
#[derive(Clone, Copy)]
enum VersionContext {
    Ensure,
    ExplicitAttach,
}

#[cfg(feature = "server")]
fn version_mismatch_message(
    hz: &HealthzInfo,
    url: &str,
    own: &str,
    context: VersionContext,
) -> Option<String> {
    match version_verdict(own, &hz.version) {
        VersionVerdict::Same => None,
        VersionVerdict::Older => Some(match context {
            VersionContext::Ensure => format!(
                "Host at {url} runs xylitol version {} (pid {}), older than this client ({own}). \
                 Restart the Host: xylitol serve stop && xylitol serve",
                hz.version, hz.pid
            ),
            VersionContext::ExplicitAttach => format!(
                "Host at {url} runs xylitol version {} (pid {}), older than this client ({own}). \
                 The local ensure path replaces an older Host; explicit attach cannot. \
                 Restart it: xylitol serve stop && xylitol serve",
                hz.version, hz.pid
            ),
        }),
        VersionVerdict::Newer => Some(format!(
            "Host at {url} runs xylitol version {} (pid {}), newer than this client ({own}). \
             Upgrade this client, or run: xylitol serve stop",
            hz.version, hz.pid
        )),
        VersionVerdict::Unparsable => Some(format!(
            "Host at {url} reports version {:?} (pid {}) that cannot be compared with this client ({own}). \
             Stop it manually: xylitol serve stop",
            hz.version, hz.pid
        )),
    }
}

#[cfg(feature = "server")]
fn fail_closed_running(
    url: &str,
    hz: HealthzInfo,
    reg: Option<xylitol_serve::Registration>,
) -> String {
    if let Some(msg) =
        version_mismatch_message(&hz, url, env!("CARGO_PKG_VERSION"), VersionContext::Ensure)
    {
        return msg;
    }
    classify_running(Some(hz), reg, url, env!("CARGO_PKG_VERSION"))
        .unwrap_or_else(|| format!("Host at {url} cannot be replaced automatically"))
}

#[cfg(feature = "server")]
fn ensure_running_message(url: &str, hz: HealthzInfo) -> String {
    version_mismatch_message(&hz, url, env!("CARGO_PKG_VERSION"), VersionContext::Ensure)
        .unwrap_or_else(|| ready_timeout_message(url))
}

#[cfg(feature = "server")]
pub fn classify_running(
    hz: Option<HealthzInfo>,
    reg: Option<xylitol_serve::Registration>,
    url: &str,
    own_version: &str,
) -> Option<String> {
    let Some(hz) = hz else {
        return Some(format!(
            "Port at {url} is occupied by a process that is not xylitol \
             (no pid/version in /healthz). Stop it or choose another port: \
             xylitol tui --port <port>"
        ));
    };
    if let Some(msg) =
        version_mismatch_message(&hz, url, own_version, VersionContext::ExplicitAttach)
    {
        return Some(msg);
    }
    if let Some(reg) = reg
        && reg.pid != hz.pid
    {
        return Some(format!(
            "note: stale registration points at pid {}, the live daemon is pid {} (proceeding)",
            reg.pid, hz.pid
        ));
    }
    None
}

#[cfg(feature = "server")]
pub fn classify_dead(
    reg: Option<xylitol_serve::Registration>,
    url: &str,
    pid_alive: bool,
) -> String {
    let Some(reg) = reg else {
        return attach_fail_message(url);
    };
    if !same_url(&reg.url, url) {
        return format!(
            "Host is not listening at {url}. The registration file points at {} — \
             no daemon at either address. Start it with: xylitol serve",
            reg.url
        );
    }
    if pid_alive {
        return format!(
            "Serve pid {} (version {}) is alive but not listening at {url} — \
             it may still be starting up. Wait a moment, or kill it: kill {}",
            reg.pid, reg.version, reg.pid
        );
    }
    format!(
        "Serve pid {} (version {}) has exited (stale registration removed). \
         Start it again with: xylitol serve",
        reg.pid, reg.version
    )
}

#[cfg(all(feature = "server", target_os = "macos"))]
unsafe extern "C" {
    fn proc_pidinfo(
        pid: libc::c_int,
        flavor: libc::c_int,
        arg: u64,
        buffer: *mut libc::c_void,
        buffersize: libc::c_int,
    ) -> libc::c_int;
}

#[cfg(feature = "server")]
pub fn pid_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        let body = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok();
        let Some(body) = body else {
            return false;
        };
        let Some((_, rest)) = body.rsplit_once(')') else {
            return true;
        };
        !matches!(rest.trim().chars().next(), Some('Z'))
    }
    #[cfg(target_os = "macos")]
    {
        let Ok(pid) = i32::try_from(pid) else {
            return false;
        };
        const PROC_PIDTBSDINFO: libc::c_int = 3;
        const PROC_BSDINFO_SIZE: usize = 136;
        const PBI_STATUS_OFFSET: usize = 4;
        let mut info = [0u8; PROC_BSDINFO_SIZE];
        let filled = unsafe {
            proc_pidinfo(
                pid,
                PROC_PIDTBSDINFO,
                0,
                info.as_mut_ptr().cast(),
                PROC_BSDINFO_SIZE as libc::c_int,
            )
        };
        if filled < (PBI_STATUS_OFFSET + size_of::<u32>()) as libc::c_int {
            return false;
        }
        let status = u32::from_ne_bytes([
            info[PBI_STATUS_OFFSET],
            info[PBI_STATUS_OFFSET + 1],
            info[PBI_STATUS_OFFSET + 2],
            info[PBI_STATUS_OFFSET + 3],
        ]);
        status != libc::SZOMB
    }
    #[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
    {
        let Ok(pid) = i32::try_from(pid) else {
            return false;
        };
        unsafe { libc::kill(pid, 0) == 0 }
    }
}

#[cfg(feature = "server")]
enum HealthzProbe {
    Info(HealthzInfo),
    Starting(HealthzInfo),
    Unauthorized,
    Unreachable,
}

#[cfg(feature = "server")]
async fn fetch_healthz(url: &str, token: Option<&str>) -> HealthzProbe {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(400))
        .build()
    else {
        return HealthzProbe::Unreachable;
    };
    let mut request = client.get(format!("{url}/healthz"));
    if let Some(token) = token {
        request = request.header(
            xylitol_protocol::auth::AUTHORIZATION_HEADER,
            xylitol_protocol::auth::basic_authorization(token),
        );
    }
    let Ok(resp) = request.send().await else {
        return HealthzProbe::Unreachable;
    };
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return HealthzProbe::Unauthorized;
    }
    let status = resp.status();
    let Ok(body) = resp.text().await else {
        return HealthzProbe::Unreachable;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else {
        return HealthzProbe::Unreachable;
    };
    let (Some(pid), Some(version)) = (
        v.get("pid").and_then(|pid| pid.as_u64()),
        v.get("version").and_then(|version| version.as_str()),
    ) else {
        return HealthzProbe::Unreachable;
    };
    let Ok(pid) = u32::try_from(pid) else {
        return HealthzProbe::Unreachable;
    };
    let info = HealthzInfo {
        pid,
        version: version.to_string(),
    };
    if status == reqwest::StatusCode::OK {
        HealthzProbe::Info(info)
    } else if status == reqwest::StatusCode::SERVICE_UNAVAILABLE
        && v.get("status").and_then(|s| s.as_str()) == Some("starting")
    {
        HealthzProbe::Starting(info)
    } else {
        HealthzProbe::Unreachable
    }
}

fn same_url(left: &str, right: &str) -> bool {
    let (Ok(left), Ok(right)) = (url::Url::parse(left), url::Url::parse(right)) else {
        return left.trim_end_matches('/') == right.trim_end_matches('/');
    };
    left.scheme() == right.scheme()
        && left.port_or_known_default() == right.port_or_known_default()
        && normalize_host(left.host_str()) == normalize_host(right.host_str())
}

fn normalize_host(host: Option<&str>) -> &str {
    match host {
        Some("0.0.0.0" | "::" | "[::]" | "localhost") => "127.0.0.1",
        Some(host) => host,
        None => "",
    }
}

#[cfg(feature = "server")]
pub fn registration_token(url: &str) -> Option<String> {
    registration_token_with(url, None)
}

#[cfg(feature = "server")]
pub fn registration_token_with(
    url: &str,
    registration_path: Option<std::path::PathBuf>,
) -> Option<String> {
    let reg_path = registration_path.unwrap_or_else(xylitol_serve::default_registration_path);
    discover_token(url, &reg_path)
}

#[cfg(feature = "server")]
fn registration_token_for(reg: &Option<xylitol_serve::Registration>, url: &str) -> Option<String> {
    reg.as_ref()
        .filter(|reg| same_url(&reg.url, url))
        .and_then(|reg| reg.token.clone())
}

#[cfg(feature = "server")]
fn discover_token(url: &str, registration_path: &std::path::Path) -> Option<String> {
    let from_reg = xylitol_serve::read_registration(registration_path)
        .ok()
        .and_then(|reg| same_url(&reg.url, url).then_some(reg.token).flatten());
    if from_reg.is_some() {
        return from_reg;
    }
    if is_loopback_url(url) && registration_path == xylitol_serve::default_registration_path() {
        return read_existing_credential();
    }
    None
}

#[cfg(feature = "server")]
fn read_existing_credential() -> Option<String> {
    let raw = std::fs::read_to_string(xylitol_serve::default_credential_path()).ok()?;
    let token = raw.trim();
    (!token.is_empty()).then(|| token.to_string())
}

#[cfg(feature = "server")]
fn dead_registration_message(
    reg: Option<&xylitol_serve::Registration>,
    url: &str,
    registration_path: &std::path::Path,
) -> String {
    let msg = classify_dead(reg.cloned(), url, reg.is_some_and(|reg| pid_alive(reg.pid)));
    if let Some(reg) = reg
        && !pid_alive(reg.pid)
    {
        xylitol_serve::remove_registration(registration_path);
    }
    msg
}

#[cfg(feature = "server")]
async fn healthz_for(
    url: &str,
    token: Option<&str>,
    registration_path: &std::path::Path,
) -> Result<Option<HealthzInfo>, String> {
    match fetch_healthz(url, token).await {
        HealthzProbe::Unauthorized => Err(auth_required_message(url, registration_path)),
        HealthzProbe::Info(hz) | HealthzProbe::Starting(hz) => Ok(Some(hz)),
        HealthzProbe::Unreachable => Ok(None),
    }
}

#[cfg(feature = "server")]
fn auth_required_message(url: &str, registration_path: &std::path::Path) -> String {
    format!(
        "Host at {url} requires authentication, but no matching credential was found in {}. \
         Restart the Host with `xylitol serve` (same account) so the registration file is rewritten.",
        registration_path.display()
    )
}

#[cfg(feature = "server")]
pub async fn attach_preflight(url: &str) -> Result<(), String> {
    attach_preflight_with(url, None).await
}

#[cfg(feature = "server")]
pub async fn attach_preflight_with(
    url: &str,
    registration_path: Option<std::path::PathBuf>,
) -> Result<(), String> {
    let reg_path = registration_path.unwrap_or_else(xylitol_serve::default_registration_path);
    let reg = xylitol_serve::read_registration(&reg_path).ok();
    let token = registration_token_for(&reg, url);
    if probe_host(url).is_err() {
        return Err(dead_registration_message(reg.as_ref(), url, &reg_path));
    }
    let hz = healthz_for(url, token.as_deref(), &reg_path).await?;
    if let Some(msg) = classify_running(hz, reg, url, env!("CARGO_PKG_VERSION")) {
        return Err(msg);
    }
    Ok(())
}

#[cfg(feature = "server")]
const READY_WAIT: Duration = Duration::from_secs(30);

#[cfg(feature = "server")]
const REPLACE_EXIT_WAIT: Duration = Duration::from_secs(5);

#[cfg(feature = "server")]
const REPLACE_LOCK_WAIT: Duration = Duration::from_secs(30);

#[cfg(feature = "server")]
pub struct EnsureHostOpts {
    pub registration_path: Option<std::path::PathBuf>,
    pub ready_timeout: Duration,
    pub lifecycle: xylitol_serve::ServeLifecycle,
    pub idle_timeout: Duration,
}

#[cfg(feature = "server")]
impl Default for EnsureHostOpts {
    fn default() -> Self {
        let (lifecycle, idle_timeout) = autostart_policy(None);
        Self {
            registration_path: None,
            ready_timeout: READY_WAIT,
            lifecycle,
            idle_timeout,
        }
    }
}

#[cfg(feature = "server")]
pub fn autostart_policy(
    config_path: Option<&std::path::Path>,
) -> (xylitol_serve::ServeLifecycle, Duration) {
    let serve = crate::config::loader::load_app_config(config_path)
        .map(|cfg| {
            cfg.section_or_default::<xylitol_official_plugin_config::types::ServeConfig>("serve")
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let lifecycle = if serve.autostart_last_client() {
        xylitol_serve::ServeLifecycle::LastClient
    } else {
        xylitol_serve::ServeLifecycle::Persistent
    };
    (lifecycle, serve.idle_duration())
}

#[cfg(feature = "server")]
pub async fn ensure_host(url: &str) -> Result<(), String> {
    ensure_host_from_config(url, None).await
}

pub async fn ensure_host_from_config(
    url: &str,
    config_path: Option<&std::path::Path>,
) -> Result<(), String> {
    let (lifecycle, idle_timeout) = autostart_policy(config_path);
    ensure_host_with(
        url,
        EnsureHostOpts {
            lifecycle,
            idle_timeout,
            ..EnsureHostOpts::default()
        },
        spawn_detached_serve,
    )
    .await
}

#[cfg(feature = "server")]
pub struct SpawnedHost {
    child: Option<std::process::Child>,
    stderr_path: Option<std::path::PathBuf>,
}

#[cfg(feature = "server")]
impl SpawnedHost {
    pub fn detached() -> Self {
        Self {
            child: None,
            stderr_path: None,
        }
    }
}

#[cfg(feature = "server")]
impl Drop for SpawnedHost {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        if let Some(path) = self.stderr_path.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(feature = "server")]
fn read_stderr_excerpt(path: &std::path::Path, limit: usize) -> String {
    let bytes = std::fs::read(path).unwrap_or_default();
    let start = bytes.len().saturating_sub(limit);
    String::from_utf8_lossy(&bytes[start..]).trim().to_string()
}

#[cfg(feature = "server")]
fn host_exited_message(url: &str, status: std::process::ExitStatus, stderr: &str) -> String {
    let mut message =
        format!("local Host exited before becoming ready at {url} (status: {status}).");
    if !stderr.is_empty() {
        message.push_str("\nHost stderr:\n");
        message.push_str(stderr);
    }
    message.push_str("\nSee ~/.xylitol/logs/xylitol.log for details.");
    message
}

#[cfg(feature = "server")]
pub async fn ensure_host_with(
    url: &str,
    opts: EnsureHostOpts,
    spawn: impl FnOnce(&str, xylitol_serve::ServeLifecycle, Duration) -> std::io::Result<SpawnedHost>,
) -> Result<(), String> {
    ensure_host_with_hooks(url, opts, spawn, crate::occupancy::subcommand::request_exit).await
}

#[cfg(feature = "server")]
async fn ensure_host_with_hooks(
    url: &str,
    opts: EnsureHostOpts,
    spawn: impl FnOnce(&str, xylitol_serve::ServeLifecycle, Duration) -> std::io::Result<SpawnedHost>,
    exit: impl Fn(u32) -> Result<(), String>,
) -> Result<(), String> {
    let reg_path = opts
        .registration_path
        .clone()
        .unwrap_or_else(xylitol_serve::default_registration_path);
    let reg = xylitol_serve::read_registration(&reg_path).ok();
    let token = discover_token(url, &reg_path);
    match inspect_live(url, token.as_deref()).await {
        LiveHost::Ready(hz) => {
            return match ready_decision(Some(&hz), env!("CARGO_PKG_VERSION")) {
                ReadyDecision::Attach => finish_running(url, hz, reg),
                ReadyDecision::FailClosed => Err(fail_closed_running(url, hz, reg)),
                ReadyDecision::Replace => replace_running(url, &reg_path, &opts, spawn, exit).await,
            };
        }
        LiveHost::Starting | LiveHost::Absent | LiveHost::Unauthorized => {}
    }
    if probe_host(url).is_err() {
        let _ = dead_registration_message(reg.as_ref(), url, &reg_path);
    }
    let mut spawned = match spawn(url, opts.lifecycle, opts.idle_timeout) {
        Ok(spawned) => spawned,
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => SpawnedHost::detached(),
        Err(err) => return Err(spawn_failed_message(url, &err.to_string())),
    };
    wait_until_ready(url, &reg_path, opts.ready_timeout, &mut spawned).await
}

#[cfg(feature = "server")]
async fn replace_running(
    url: &str,
    registration_path: &std::path::Path,
    opts: &EnsureHostOpts,
    spawn: impl FnOnce(&str, xylitol_serve::ServeLifecycle, Duration) -> std::io::Result<SpawnedHost>,
    exit: impl Fn(u32) -> Result<(), String>,
) -> Result<(), String> {
    let _lock = acquire_replace_lock(registration_path).await?;
    let token = discover_token(url, registration_path);
    let target = match inspect_live(url, token.as_deref()).await {
        LiveHost::Ready(current) => match ready_decision(Some(&current), env!("CARGO_PKG_VERSION"))
        {
            ReadyDecision::Attach => {
                return finish_running(
                    url,
                    current,
                    xylitol_serve::read_registration(registration_path).ok(),
                );
            }
            ReadyDecision::FailClosed => {
                return Err(fail_closed_running(url, current, None));
            }
            ReadyDecision::Replace => Some(current),
        },
        LiveHost::Starting | LiveHost::Absent | LiveHost::Unauthorized => None,
    };
    if let Some(target) = target {
        exit(target.pid)?;
        wait_for_exit(target.pid, REPLACE_EXIT_WAIT).await?;
        if let Ok(reg) = xylitol_serve::read_registration(registration_path)
            && reg.pid == target.pid
        {
            xylitol_serve::remove_registration(registration_path);
        }
    }
    let mut spawned = match spawn(url, opts.lifecycle, opts.idle_timeout) {
        Ok(spawned) => spawned,
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => SpawnedHost::detached(),
        Err(err) => return Err(spawn_failed_message(url, &err.to_string())),
    };
    wait_until_ready(url, registration_path, opts.ready_timeout, &mut spawned).await
}

#[cfg(all(feature = "server", unix))]
fn replace_lock_path(registration_path: &std::path::Path) -> std::path::PathBuf {
    let mut path = registration_path.as_os_str().to_os_string();
    path.push(".replace.lock");
    std::path::PathBuf::from(path)
}

#[cfg(feature = "server")]
#[cfg_attr(not(unix), allow(dead_code))]
struct ReplaceLock {
    #[cfg(unix)]
    _file: std::fs::File,
}

#[cfg(feature = "server")]
async fn acquire_replace_lock(registration_path: &std::path::Path) -> Result<ReplaceLock, String> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        let lock_path = replace_lock_path(registration_path);
        let display = lock_path.display().to_string();
        let file = tokio::task::spawn_blocking(move || -> Result<std::fs::File, String> {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&lock_path)
                .map_err(|err| format!("Failed to open Host replace lock {display}: {err}"))?;
            let deadline = std::time::Instant::now() + REPLACE_LOCK_WAIT;
            loop {
                let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
                if rc == 0 {
                    return Ok(file);
                }
                let err = std::io::Error::last_os_error();
                if err.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(format!("Failed to lock {display}: {err}"));
                }
                if std::time::Instant::now() >= deadline {
                    return Err(format!(
                        "Another client is replacing the Host (lock {display}); retry shortly."
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        })
        .await
        .map_err(|err| format!("Host replace lock task failed: {err}"))??;
        Ok(ReplaceLock { _file: file })
    }
    #[cfg(not(unix))]
    {
        let _ = registration_path;
        Err(
            "Replacing an older Host is not supported on this platform; \
             run `xylitol serve stop` and restart."
                .into(),
        )
    }
}

#[cfg(feature = "server")]
pub fn replace_timeout_message(pid: u32, timeout: Duration) -> String {
    format!(
        "Host pid {pid} did not exit within {}s after SIGTERM. \
         Stop it manually (kill {pid} or xylitol serve stop), then retry.",
        timeout.as_secs()
    )
}

#[cfg(feature = "server")]
async fn wait_for_exit(pid: u32, timeout: Duration) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + timeout;
    while pid_alive(pid) {
        if tokio::time::Instant::now() >= deadline {
            return Err(replace_timeout_message(pid, timeout));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Ok(())
}

#[cfg(feature = "server")]
enum LiveHost {
    Ready(HealthzInfo),
    Starting,
    Absent,
    Unauthorized,
}

#[cfg(feature = "server")]
async fn inspect_live(url: &str, token: Option<&str>) -> LiveHost {
    if probe_host(url).is_err() {
        return LiveHost::Absent;
    }
    match fetch_healthz(url, token).await {
        HealthzProbe::Unauthorized => LiveHost::Unauthorized,
        HealthzProbe::Info(hz) => LiveHost::Ready(hz),
        HealthzProbe::Starting(_) => LiveHost::Starting,
        HealthzProbe::Unreachable => LiveHost::Absent,
    }
}

#[cfg(feature = "server")]
fn finish_running(
    url: &str,
    hz: HealthzInfo,
    reg: Option<xylitol_serve::Registration>,
) -> Result<(), String> {
    match classify_running(Some(hz), reg, url, env!("CARGO_PKG_VERSION")) {
        Some(msg) => Err(msg),
        None => Ok(()),
    }
}

#[cfg(feature = "server")]
async fn wait_until_ready(
    url: &str,
    registration_path: &std::path::Path,
    timeout: Duration,
    spawned: &mut SpawnedHost,
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some(child) = spawned.child.as_mut()
            && let Ok(Some(status)) = child.try_wait()
        {
            spawned.child = None;
            let stderr = spawned
                .stderr_path
                .as_deref()
                .map(|path| read_stderr_excerpt(path, 4096))
                .unwrap_or_default();
            return Err(host_exited_message(url, status, &stderr));
        }
        let token = discover_token(url, registration_path);
        match fetch_healthz(url, token.as_deref()).await {
            HealthzProbe::Info(hz) => {
                return finish_running(
                    url,
                    hz,
                    xylitol_serve::read_registration(registration_path).ok(),
                );
            }
            HealthzProbe::Unauthorized | HealthzProbe::Starting(_) | HealthzProbe::Unreachable => {}
        }
        if tokio::time::Instant::now() >= deadline {
            let token = discover_token(url, registration_path);
            if probe_host(url).is_ok() {
                match fetch_healthz(url, token.as_deref()).await {
                    HealthzProbe::Unauthorized => {
                        return Err(auth_required_message(url, registration_path));
                    }
                    HealthzProbe::Info(hz) | HealthzProbe::Starting(hz) => {
                        return Err(ensure_running_message(url, hz));
                    }
                    HealthzProbe::Unreachable => {
                        if let Some(msg) =
                            classify_running(None, None, url, env!("CARGO_PKG_VERSION"))
                        {
                            return Err(msg);
                        }
                    }
                }
            }
            return Err(ready_timeout_message(url));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

#[cfg(feature = "server")]
pub fn spawn_detached_serve(
    url: &str,
    lifecycle: xylitol_serve::ServeLifecycle,
    idle_timeout: Duration,
) -> std::io::Result<SpawnedHost> {
    let parsed = url::Url::parse(url).map_err(std::io::Error::other)?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port = parsed.port_or_known_default().unwrap_or(18790);
    let exe = std::env::current_exe()?;
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let stderr_path = std::env::temp_dir().join(format!(
        "xylitol-host-stderr-{}-{unique}.log",
        std::process::id()
    ));
    let mut stderr_opts = std::fs::OpenOptions::new();
    stderr_opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        stderr_opts.mode(0o600);
    }
    let stderr_file = stderr_opts.open(&stderr_path)?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("serve")
        .arg("--host")
        .arg(host)
        .arg("--port")
        .arg(port.to_string())
        .arg("--lifecycle")
        .arg(lifecycle.as_str())
        .arg("--idle-timeout")
        .arg(xylitol_serve::format_idle_timeout(idle_timeout))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::from(stderr_file));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
    let child = cmd.spawn()?;
    Ok(SpawnedHost {
        child: Some(child),
        stderr_path: Some(stderr_path),
    })
}

#[cfg(all(test, feature = "server"))]
mod c2475_tests {
    use super::*;

    fn reg(url: &str, pid: u32) -> Option<xylitol_serve::Registration> {
        Some(xylitol_serve::Registration {
            url: url.into(),
            pid,
            version: "0.0.0-dev".into(),
            token: None,
            lifecycle: None,
        })
    }

    #[test]
    fn classify_running_blocks_non_xylitol_occupant() {
        let msg = classify_running(None, reg("http://u", 1), "http://u", "0.0.0-dev");
        let msg = msg.expect("must block");
        assert!(msg.contains("not xylitol"), "{msg}");
    }

    #[test]
    fn classify_running_blocks_version_mismatch() {
        let hz = HealthzInfo {
            pid: 7,
            version: "0.0.9".into(),
        };
        let msg = classify_running(Some(hz), reg("http://u", 7), "http://u", "0.0.0-dev");
        let msg = msg.expect("must block");
        assert!(msg.contains("version"), "{msg}");
    }

    #[test]
    fn classify_running_warns_on_stale_registration_but_proceeds() {
        let hz = HealthzInfo {
            pid: 9,
            version: "0.0.0-dev".into(),
        };
        let msg = classify_running(
            Some(hz.clone()),
            reg("http://u", 5),
            "http://u",
            "0.0.0-dev",
        );
        let msg = msg.expect("stale registration must warn");
        assert!(msg.contains("stale registration"), "{msg}");
        assert!(classify_running(Some(hz), reg("http://u", 9), "http://u", "0.0.0-dev").is_none());
    }

    #[test]
    fn classify_dead_branches() {
        let no_reg = classify_dead(None, "http://u", false);
        assert!(no_reg.contains("not listening"), "{no_reg}");

        let alive = classify_dead(reg("http://u", 42), "http://u", true);
        assert!(
            alive.contains("pid 42") && alive.contains("starting up"),
            "{alive}"
        );

        let dead = classify_dead(reg("http://u", 42), "http://u", false);
        assert!(
            dead.contains("has exited") && dead.contains("pid 42"),
            "{dead}"
        );

        let mismatch = classify_dead(reg("http://other", 42), "http://u", false);
        assert!(mismatch.contains("http://other"), "{mismatch}");
    }

    #[tokio::test]
    async fn attach_preflight_reports_auth_required() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(wiremock::ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let url = server.uri();
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("serve.json");
        xylitol_serve::write_registration(
            &path,
            &xylitol_serve::Registration {
                url: url.clone(),
                pid: 1,
                version: "0.0.0-dev".into(),
                token: Some("secret-token".into()),
                lifecycle: None,
            },
        )
        .expect("write registration");

        let msg = attach_preflight_with(&url, Some(path))
            .await
            .expect_err("401 must fail preflight");
        assert!(msg.contains("requires authentication"), "{msg}");
        assert!(msg.contains("xylitol serve"), "{msg}");
    }

    #[test]
    fn dead_registration_message_removes_stale_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("serve.json");
        let reg = xylitol_serve::Registration {
            url: "http://127.0.0.1:1".into(),
            pid: 2_147_483_647,
            version: "0.0.0-dev".into(),
            token: None,
            lifecycle: None,
        };
        xylitol_serve::write_registration(&path, &reg).expect("write");
        let msg = dead_registration_message(Some(&reg), "http://127.0.0.1:1", &path);
        assert!(msg.contains("has exited"), "{msg}");
        assert!(!path.exists(), "stale registration must be removed");
    }

    #[test]
    fn auth_required_message_is_actionable() {
        let msg = auth_required_message(
            "http://127.0.0.1:18790",
            std::path::Path::new("/tmp/serve.json"),
        );
        assert!(msg.contains("requires authentication"), "{msg}");
        assert!(msg.contains("xylitol serve"), "{msg}");
        assert!(msg.contains("serve.json"), "{msg}");
    }

    #[test]
    fn registration_token_only_matches_target_url() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("serve.json");
        let mut reg = reg("http://127.0.0.1:18790", 7).expect("reg");
        reg.token = Some("secret-token".into());
        xylitol_serve::write_registration(&path, &reg).expect("write");

        assert_eq!(
            registration_token_with("http://127.0.0.1:18790", Some(path.clone())).as_deref(),
            Some("secret-token")
        );
        assert!(
            registration_token_with("http://127.0.0.1:18791", Some(path.clone())).is_none(),
            "token must not be sent to a different URL"
        );
        assert_eq!(
            registration_token_with("http://localhost:18790", Some(path.clone())).as_deref(),
            Some("secret-token"),
            "localhost and loopback must be equivalent"
        );
        assert_eq!(
            registration_token_with("http://0.0.0.0:18790", Some(path)).as_deref(),
            Some("secret-token"),
            "wildcard bind and loopback must be equivalent"
        );
    }

    fn healthz_ok() -> wiremock::ResponseTemplate {
        wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "pid": 3,
            "version": env!("CARGO_PKG_VERSION"),
        }))
    }

    #[tokio::test]
    async fn ensure_host_skips_spawn_when_ready() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(healthz_ok())
            .mount(&server)
            .await;
        let spawned = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = spawned.clone();
        let dir = tempfile::tempdir().expect("tempdir");
        ensure_host_with(
            &server.uri(),
            EnsureHostOpts {
                registration_path: Some(dir.path().join("serve.json")),
                ready_timeout: Duration::from_millis(200),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            move |_, _, _| {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(SpawnedHost::detached())
            },
        )
        .await
        .expect("ready host");
        assert!(
            !spawned.load(std::sync::atomic::Ordering::SeqCst),
            "must not spawn when already ready"
        );
    }

    #[tokio::test]
    async fn ensure_host_spawns_when_absent_then_times_out() {
        let spawned = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = spawned.clone();
        let dir = tempfile::tempdir().expect("tempdir");
        let err = ensure_host_with(
            "http://127.0.0.1:1",
            EnsureHostOpts {
                registration_path: Some(dir.path().join("serve.json")),
                ready_timeout: Duration::from_millis(80),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            move |_, _, _| {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(SpawnedHost::detached())
            },
        )
        .await
        .expect_err("absent host times out");
        assert!(spawned.load(std::sync::atomic::Ordering::SeqCst));
        assert!(err.contains("Timed out"), "{err}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn ensure_host_reports_child_exit_before_ready() {
        let dir = tempfile::tempdir().expect("tempdir");
        let stderr_path = dir.path().join("host.err");
        std::fs::write(&stderr_path, "config section boom\n").expect("write stderr");
        let child = std::process::Command::new("sh")
            .arg("-c")
            .arg("exit 3")
            .spawn()
            .expect("spawn sh");
        let err = ensure_host_with(
            "http://127.0.0.1:1",
            EnsureHostOpts {
                registration_path: Some(dir.path().join("serve.json")),
                ready_timeout: Duration::from_secs(5),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            move |_, _, _| {
                Ok(SpawnedHost {
                    child: Some(child),
                    stderr_path: Some(stderr_path),
                })
            },
        )
        .await
        .expect_err("child exit must fail fast");
        assert!(err.contains("exited before becoming ready"), "{err}");
        assert!(err.contains("config section boom"), "{err}");
    }

    #[tokio::test]
    async fn ensure_host_does_not_spawn_when_host_is_newer() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok",
                    "pid": 3,
                    "version": "9.9.9",
                })),
            )
            .mount(&server)
            .await;
        let spawned = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = spawned.clone();
        let dir = tempfile::tempdir().expect("tempdir");
        let err = ensure_host_with(
            &server.uri(),
            EnsureHostOpts {
                registration_path: Some(dir.path().join("serve.json")),
                ready_timeout: Duration::from_millis(200),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            move |_, _, _| {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(SpawnedHost::detached())
            },
        )
        .await
        .expect_err("version mismatch");
        assert!(!spawned.load(std::sync::atomic::Ordering::SeqCst));
        assert!(err.contains("version"), "{err}");
    }

    #[tokio::test]
    async fn ensure_host_retries_auth_until_registration_token_appears() {
        let server = wiremock::MockServer::start().await;
        let token = "spawn-token";
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .and(wiremock::matchers::header(
                "authorization",
                xylitol_protocol::auth::basic_authorization(token),
            ))
            .respond_with(healthz_ok())
            .with_priority(1)
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(wiremock::ResponseTemplate::new(401))
            .with_priority(5)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("serve.json");
        let url = server.uri();
        let spawned = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = spawned.clone();
        let path_for_spawn = path.clone();
        let url_for_spawn = url.clone();
        ensure_host_with(
            &url,
            EnsureHostOpts {
                registration_path: Some(path),
                ready_timeout: Duration::from_millis(800),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            move |_, _, _| {
                xylitol_serve::write_registration(
                    &path_for_spawn,
                    &xylitol_serve::Registration {
                        url: url_for_spawn.clone(),
                        pid: 3,
                        version: env!("CARGO_PKG_VERSION").into(),
                        token: Some(token.into()),
                        lifecycle: None,
                    },
                )
                .expect("write");
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(SpawnedHost::detached())
            },
        )
        .await
        .expect("ensure recovers token after spawn");
        assert!(spawned.load(std::sync::atomic::Ordering::SeqCst));
    }
}

#[cfg(all(test, feature = "server"))]
mod c3040_tests {
    use super::*;

    fn healthz_ok() -> wiremock::ResponseTemplate {
        wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "ok",
            "pid": 3,
            "version": env!("CARGO_PKG_VERSION"),
        }))
    }

    #[test]
    fn version_verdict_orders_semver() {
        assert_eq!(version_verdict("0.2.0", "0.2.0"), VersionVerdict::Same);
        assert_eq!(version_verdict("0.2.0", "0.1.9"), VersionVerdict::Older);
        assert_eq!(version_verdict("0.2.0", "0.3.0"), VersionVerdict::Newer);
        assert_eq!(version_verdict("0.2.0", "0.2.0-pre"), VersionVerdict::Older);
        assert_eq!(version_verdict("0.2.0-pre", "0.2.0"), VersionVerdict::Newer);
        assert_eq!(
            version_verdict("0.2.0", "not-a-version"),
            VersionVerdict::Unparsable
        );
        assert_eq!(version_verdict("nope", "0.2.0"), VersionVerdict::Unparsable);
    }

    #[test]
    fn ready_decision_covers_replace_and_fail_closed() {
        let older = HealthzInfo {
            pid: 1,
            version: "0.1.0".into(),
        };
        let same = HealthzInfo {
            pid: 2,
            version: "0.2.0".into(),
        };
        let newer = HealthzInfo {
            pid: 3,
            version: "0.3.0".into(),
        };
        let unparsable = HealthzInfo {
            pid: 4,
            version: "weird".into(),
        };
        assert_eq!(ready_decision(None, "0.2.0"), ReadyDecision::FailClosed);
        assert_eq!(
            ready_decision(Some(&older), "0.2.0"),
            ReadyDecision::Replace
        );
        assert_eq!(ready_decision(Some(&same), "0.2.0"), ReadyDecision::Attach);
        assert_eq!(
            ready_decision(Some(&newer), "0.2.0"),
            ReadyDecision::FailClosed
        );
        assert_eq!(
            ready_decision(Some(&unparsable), "0.2.0"),
            ReadyDecision::FailClosed
        );
    }

    #[tokio::test]
    async fn wait_for_exit_returns_when_pid_is_gone() {
        wait_for_exit(u32::MAX, Duration::from_millis(100))
            .await
            .expect("dead pid must not block");
    }

    #[tokio::test]
    async fn wait_for_exit_times_out_for_live_pid() {
        let pid = std::process::id();
        let err = wait_for_exit(pid, Duration::from_millis(80))
            .await
            .expect_err("live pid must time out");
        assert!(err.contains(&pid.to_string()), "{err}");
        assert!(err.contains("SIGTERM"), "{err}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn replace_lock_serializes_clients() {
        let dir = tempfile::tempdir().expect("tempdir");
        let reg_path = dir.path().join("serve.json");
        let first = acquire_replace_lock(&reg_path).await.expect("first lock");
        let path = reg_path.clone();
        let second = tokio::spawn(async move { acquire_replace_lock(&path).await });
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(
            !second.is_finished(),
            "second client must wait for the replace lock"
        );
        drop(first);
        let second = tokio::time::timeout(Duration::from_secs(2), second)
            .await
            .expect("second acquire timed out")
            .expect("join")
            .expect("second lock");
        drop(second);
    }

    #[tokio::test]
    async fn replace_running_replaces_older_host() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok",
                    "pid": u32::MAX,
                    "version": "0.0.0-aa",
                })),
            )
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(healthz_ok())
            .with_priority(5)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().expect("tempdir");
        let reg_path = dir.path().join("serve.json");
        xylitol_serve::write_registration(
            &reg_path,
            &xylitol_serve::Registration {
                url: server.uri(),
                pid: u32::MAX,
                version: "0.0.0-aa".into(),
                token: None,
                lifecycle: None,
            },
        )
        .expect("write registration");

        let exited = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let exited_for_closure = exited.clone();
        let spawned = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = spawned.clone();
        let spawn_reg_path = reg_path.clone();
        let spawn_url = server.uri();
        replace_running(
            &server.uri(),
            &reg_path,
            &EnsureHostOpts {
                registration_path: Some(reg_path.clone()),
                ready_timeout: Duration::from_millis(800),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            move |_, _, _| {
                xylitol_serve::write_registration(
                    &spawn_reg_path,
                    &xylitol_serve::Registration {
                        url: spawn_url.clone(),
                        pid: 3,
                        version: env!("CARGO_PKG_VERSION").into(),
                        token: None,
                        lifecycle: None,
                    },
                )
                .expect("write new registration");
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(SpawnedHost::detached())
            },
            move |pid| {
                exited_for_closure.store(pid, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
        )
        .await
        .expect("replace older host");
        assert_eq!(exited.load(std::sync::atomic::Ordering::SeqCst), u32::MAX);
        assert!(spawned.load(std::sync::atomic::Ordering::SeqCst));
        let reg = xylitol_serve::read_registration(&reg_path).expect("new registration");
        assert_eq!(reg.pid, 3, "new registration must point at the fresh Host");
        assert_eq!(reg.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn ensure_host_replaces_older_host() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok",
                    "pid": u32::MAX,
                    "version": "0.0.0-aa",
                })),
            )
            .up_to_n_times(2)
            .with_priority(1)
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(healthz_ok())
            .with_priority(5)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().expect("tempdir");
        let reg_path = dir.path().join("serve.json");
        xylitol_serve::write_registration(
            &reg_path,
            &xylitol_serve::Registration {
                url: server.uri(),
                pid: u32::MAX,
                version: "0.0.0-aa".into(),
                token: None,
                lifecycle: None,
            },
        )
        .expect("write registration");
        let exited = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let exited_for_closure = exited.clone();
        let spawned = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = spawned.clone();
        let spawn_reg_path = reg_path.clone();
        let spawn_url = server.uri();
        ensure_host_with_hooks(
            &server.uri(),
            EnsureHostOpts {
                registration_path: Some(reg_path.clone()),
                ready_timeout: Duration::from_millis(800),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            move |_, _, _| {
                xylitol_serve::write_registration(
                    &spawn_reg_path,
                    &xylitol_serve::Registration {
                        url: spawn_url.clone(),
                        pid: 3,
                        version: env!("CARGO_PKG_VERSION").into(),
                        token: None,
                        lifecycle: None,
                    },
                )
                .expect("write new registration");
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(SpawnedHost::detached())
            },
            move |pid| {
                exited_for_closure.store(pid, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
        )
        .await
        .expect("ensure replaces older host");
        assert_eq!(exited.load(std::sync::atomic::Ordering::SeqCst), u32::MAX);
        assert!(spawned.load(std::sync::atomic::Ordering::SeqCst));
        let reg = xylitol_serve::read_registration(&reg_path).expect("new registration");
        assert_eq!(reg.pid, 3);
    }

    #[tokio::test]
    async fn ensure_host_reports_foreign_occupant_after_ready_timeout() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().expect("tempdir");
        let spawned = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = spawned.clone();
        let err = ensure_host_with(
            &server.uri(),
            EnsureHostOpts {
                registration_path: Some(dir.path().join("serve.json")),
                ready_timeout: Duration::from_millis(200),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            move |_, _, _| {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                Err(std::io::Error::from(std::io::ErrorKind::AddrInUse))
            },
        )
        .await
        .expect_err("foreign occupant must fail closed");
        assert!(err.contains("not xylitol"), "{err}");
        assert!(err.contains("xylitol tui --port"), "{err}");
    }

    #[tokio::test]
    async fn ensure_host_rejects_unparsable_version() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok",
                    "pid": 3,
                    "version": "weird",
                })),
            )
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().expect("tempdir");
        let err = ensure_host_with(
            &server.uri(),
            EnsureHostOpts {
                registration_path: Some(dir.path().join("serve.json")),
                ready_timeout: Duration::from_millis(200),
                lifecycle: xylitol_serve::ServeLifecycle::LastClient,
                idle_timeout: Duration::from_millis(30),
            },
            |_, _, _| Ok(SpawnedHost::detached()),
        )
        .await
        .expect_err("unparsable version must fail closed");
        assert!(err.contains("cannot be compared"), "{err}");
    }

    #[tokio::test]
    async fn concurrent_replace_requests_only_one_exit() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok",
                    "pid": u32::MAX,
                    "version": "0.0.0-aa",
                })),
            )
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(healthz_ok())
            .with_priority(5)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().expect("tempdir");
        let reg_path = dir.path().join("serve.json");
        xylitol_serve::write_registration(
            &reg_path,
            &xylitol_serve::Registration {
                url: server.uri(),
                pid: u32::MAX,
                version: "0.0.0-aa".into(),
                token: None,
                lifecycle: None,
            },
        )
        .expect("write registration");
        let opts = EnsureHostOpts {
            registration_path: Some(reg_path.clone()),
            ready_timeout: Duration::from_millis(800),
            lifecycle: xylitol_serve::ServeLifecycle::LastClient,
            idle_timeout: Duration::from_millis(30),
        };
        let exits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let spawns = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let exits_a = exits.clone();
        let exits_b = exits.clone();
        let spawns_a = spawns.clone();
        let spawns_b = spawns.clone();
        let url = server.uri();
        let first = replace_running(
            &url,
            &reg_path,
            &opts,
            move |_, _, _| {
                spawns_a.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(SpawnedHost::detached())
            },
            move |_| {
                exits_a.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
        );
        let second = replace_running(
            &url,
            &reg_path,
            &opts,
            move |_, _, _| {
                spawns_b.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(SpawnedHost::detached())
            },
            move |_| {
                exits_b.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            },
        );
        let (first, second) = tokio::join!(first, second);
        first.expect("first replace");
        second.expect("second replace");
        assert_eq!(
            exits.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "only one client may signal the older Host"
        );
        assert_eq!(spawns.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
