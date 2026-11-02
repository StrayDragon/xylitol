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
    if hz.version != own_version {
        return Some(format!(
            "Host at {url} runs xylitol version {} but this client is {own_version}. \
             Restart the daemon: kill {} && xylitol serve",
            hz.version, hz.pid
        ));
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
    #[cfg(not(target_os = "linux"))]
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
        .map(|cfg| cfg.serve)
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
pub async fn ensure_host_with(
    url: &str,
    opts: EnsureHostOpts,
    spawn: impl FnOnce(&str, xylitol_serve::ServeLifecycle, Duration) -> std::io::Result<()>,
) -> Result<(), String> {
    let reg_path = opts
        .registration_path
        .clone()
        .unwrap_or_else(xylitol_serve::default_registration_path);
    let reg = xylitol_serve::read_registration(&reg_path).ok();
    let token = discover_token(url, &reg_path);
    match inspect_live(url, token.as_deref()).await {
        LiveHost::Ready(hz) => return finish_running(url, hz, reg),
        LiveHost::Starting | LiveHost::Absent | LiveHost::Unauthorized => {}
    }
    if probe_host(url).is_err() {
        let _ = dead_registration_message(reg.as_ref(), url, &reg_path);
    }
    if let Err(err) = spawn(url, opts.lifecycle, opts.idle_timeout)
        && err.kind() != std::io::ErrorKind::AddrInUse
    {
        return Err(spawn_failed_message(url, &err.to_string()));
    }
    wait_until_ready(url, &reg_path, opts.ready_timeout).await
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
) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
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
                        if let Some(msg) =
                            classify_running(Some(hz), None, url, env!("CARGO_PKG_VERSION"))
                        {
                            return Err(msg);
                        }
                    }
                    HealthzProbe::Unreachable => {}
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
) -> std::io::Result<()> {
    let parsed = url::Url::parse(url).map_err(std::io::Error::other)?;
    let host = parsed.host_str().unwrap_or("127.0.0.1");
    let port = parsed.port_or_known_default().unwrap_or(18790);
    let exe = std::env::current_exe()?;
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
        .stderr(std::process::Stdio::null());
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
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
    });
    Ok(())
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
                Ok(())
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
                Ok(())
            },
        )
        .await
        .expect_err("absent host times out");
        assert!(spawned.load(std::sync::atomic::Ordering::SeqCst));
        assert!(err.contains("Timed out"), "{err}");
    }

    #[tokio::test]
    async fn ensure_host_does_not_spawn_on_version_mismatch() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/healthz"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok",
                    "pid": 3,
                    "version": "0.0.0-other",
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
                Ok(())
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
                Ok(())
            },
        )
        .await
        .expect("ensure recovers token after spawn");
        assert!(spawned.load(std::sync::atomic::Ordering::SeqCst));
    }
}
