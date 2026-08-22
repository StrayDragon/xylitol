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
