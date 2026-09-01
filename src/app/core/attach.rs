//! Product TUI attach: default Host URL and liveness probe.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use thiserror::Error;

/// Product TUI default attach origin (scheme + host + port).
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

/// Probe that a TCP listener is accepting at the attach URL.
///
/// This is liveness, not `host.describe`. A listening process that does not
/// speak the four-quadrant envelope still counts as "in listening" here;
/// handshake mismatch is a later error.
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

// ---- c2475 sr-reg1：attach 预检与注册文件分级诊断 ----

/// Healthz identity snapshot.
#[cfg(feature = "server")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthzInfo {
    pub pid: u32,
    pub version: String,
}

/// Pure classification of an attach whose TCP probe succeeded. `None` = proceed.
/// Occupant without xylitol identity → blocked; version mismatch → blocked;
/// stale registration (live pid differs from file) → informational message.
#[cfg(feature = "server")]
pub fn classify_running(
    hz: Option<HealthzInfo>,
    reg: Option<crate::app::server::registration::Registration>,
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

/// Pure classification of a failed TCP probe against the registration file.
#[cfg(feature = "server")]
pub fn classify_dead(
    reg: Option<crate::app::server::registration::Registration>,
    url: &str,
    pid_alive: bool,
) -> String {
    let Some(reg) = reg else {
        return attach_fail_message(url);
    };
    if reg.url != url {
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
        true
    }
}

#[cfg(feature = "server")]
async fn fetch_healthz(url: &str) -> Option<HealthzInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(400))
        .build()
        .ok()?;
    let body = client
        .get(format!("{url}/healthz"))
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    Some(HealthzInfo {
        pid: u32::try_from(v.get("pid")?.as_u64()?).ok()?,
        version: v.get("version")?.as_str()?.to_string(),
    })
}

/// Attach preflight (c2475): TCP probe, then registration/healthz
/// classification. `Err(message)` stops the attach with an actionable error.
#[cfg(feature = "server")]
pub async fn attach_preflight(url: &str) -> Result<(), String> {
    attach_preflight_with(url, None).await
}

/// Same preflight with an explicit registration path (BDD / tests).
#[cfg(feature = "server")]
pub async fn attach_preflight_with(
    url: &str,
    registration_path: Option<std::path::PathBuf>,
) -> Result<(), String> {
    let reg_path = registration_path
        .unwrap_or_else(crate::app::server::registration::default_registration_path);
    let reg = crate::app::server::registration::read_registration(&reg_path).ok();
    if probe_host(url).is_err() {
        let msg = classify_dead(
            reg.clone(),
            url,
            reg.as_ref().is_some_and(|r| pid_alive(r.pid)),
        );
        if let Some(r) = &reg
            && !pid_alive(r.pid)
        {
            crate::app::server::registration::remove_registration(&reg_path);
        }
        return Err(msg);
    }
    let hz = fetch_healthz(url).await;
    if let Some(msg) = classify_running(hz, reg, url, env!("CARGO_PKG_VERSION")) {
        return Err(msg);
    }
    Ok(())
}

#[cfg(all(test, feature = "server"))]
mod c2475_tests {
    use super::*;

    fn reg(url: &str, pid: u32) -> Option<crate::app::server::registration::Registration> {
        Some(crate::app::server::registration::Registration {
            url: url.into(),
            pid,
            version: "0.0.0-dev".into(),
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
}
