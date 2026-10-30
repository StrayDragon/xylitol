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
fn pid_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
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
    HealthzProbe::Info(HealthzInfo {
        pid,
        version: version.to_string(),
    })
}

#[cfg(feature = "server")]
fn same_url(left: &str, right: &str) -> bool {
    let (Ok(left), Ok(right)) = (url::Url::parse(left), url::Url::parse(right)) else {
        return left.trim_end_matches('/') == right.trim_end_matches('/');
    };
    left.scheme() == right.scheme()
        && left.port_or_known_default() == right.port_or_known_default()
        && normalize_host(left.host_str()) == normalize_host(right.host_str())
}

#[cfg(feature = "server")]
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
    let reg = xylitol_serve::read_registration(&reg_path).ok()?;
    same_url(&reg.url, url).then_some(reg.token).flatten()
}

#[cfg(feature = "server")]
fn registration_token_for(reg: &Option<xylitol_serve::Registration>, url: &str) -> Option<String> {
    reg.as_ref()
        .filter(|reg| same_url(&reg.url, url))
        .and_then(|reg| reg.token.clone())
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
        HealthzProbe::Info(hz) => Ok(Some(hz)),
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

#[cfg(all(test, feature = "server"))]
mod c2475_tests {
    use super::*;

    fn reg(url: &str, pid: u32) -> Option<xylitol_serve::Registration> {
        Some(xylitol_serve::Registration {
            url: url.into(),
            pid,
            version: "0.0.0-dev".into(),
            token: None,
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
}
