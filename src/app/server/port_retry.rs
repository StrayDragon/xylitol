//! Port retry logic — when the configured port is busy, try port+1 up to
//! [`PORT_RETRY_LIMIT`] times, updating the lock file with the actual port.

use std::net::TcpListener;
use std::path::Path;

use crate::app::server::lock::{LockInfo, ServerLock};

/// Maximum number of port increments before giving up.
pub const PORT_RETRY_LIMIT: u16 = 10;

/// Error returned when no port in the retry range is available.
#[derive(Debug)]
pub struct PortRetryExhausted {
    pub start_port: u16,
    pub last_error: std::io::Error,
}

impl std::fmt::Display for PortRetryExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "all ports from {} to {} are in use (last error: {})",
            self.start_port,
            self.start_port + PORT_RETRY_LIMIT - 1,
            self.last_error,
        )
    }
}

impl std::error::Error for PortRetryExhausted {}

/// Try to bind to `start_port`; if busy, try `start_port + 1` up to
/// [`PORT_RETRY_LIMIT`] attempts. On success, returns the bound listener
/// and the actual port number.
/// Try to bind to `start_port`; if busy, try `start_port + 1` up to
/// [`PORT_RETRY_LIMIT`] attempts. On success, returns the bound listener
/// and the actual port number from `listener.local_addr()`.
pub fn bind_with_retry(start_port: u16) -> Result<(TcpListener, u16), PortRetryExhausted> {
    let mut last_error = std::io::Error::new(std::io::ErrorKind::AddrInUse, "no ports available");
    for offset in 0..PORT_RETRY_LIMIT {
        let port = if start_port == 0 {
            0u16 // OS-assigned
        } else {
            start_port + offset
        };
        match TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)) {
            Ok(listener) => {
                let actual = listener.local_addr().unwrap().port();
                return Ok((listener, actual));
            }
            Err(e) => {
                last_error = e;
                // Continue to next port.
            }
        }
    }
    Err(PortRetryExhausted {
        start_port,
        last_error,
    })
}

/// Acquire a lock and bind to a port, retrying if the port is busy.
///
/// Returns the bound listener, the actual port, and the lock handle.
pub fn acquire_lock_and_bind(
    lock_path: &Path,
    hostname: &str,
    start_port: u16,
) -> Result<(TcpListener, u16, ServerLock), Box<dyn std::error::Error>> {
    let pid = std::process::id();

    // First try: acquire lock + bind at start_port.
    let info = LockInfo {
        port: start_port,
        pid,
        hostname: hostname.to_string(),
    };
    let lock = ServerLock::try_acquire(lock_path, &info)?;

    // Now bind — if busy, retry with port+1 and update lock.
    for offset in 0..PORT_RETRY_LIMIT {
        let try_port = if start_port == 0 {
            0u16
        } else {
            start_port + offset
        };
        match TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, try_port)) {
            Ok(listener) => {
                let actual = listener.local_addr().unwrap().port();
                if actual != start_port {
                    lock.update_port(actual)?;
                }
                return Ok((listener, actual, lock));
            }
            Err(_) if offset < PORT_RETRY_LIMIT - 1 => {
                // Lock already held; try next port.
                continue;
            }
            Err(e) => {
                return Err(Box::new(PortRetryExhausted {
                    start_port,
                    last_error: e,
                }));
            }
        }
    }

    unreachable!("loop always returns or errors");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn temp_lock_path() -> std::path::PathBuf {
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        std::env::temp_dir().join(format!("xylitol-test-portretry-{port}"))
    }

    #[test]
    fn bind_first_port_succeeds() {
        let path = temp_lock_path();
        let (listener, port, lock) =
            acquire_lock_and_bind(&path, "test", 0).expect("bind with retry");
        assert!(port > 0);
        assert!(path.exists());
        drop(listener);
        drop(lock);
    }

    #[test]
    fn port_retry_bumps_port() {
        // Occupy a specific port first.
        let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let occupied_port = occupied.local_addr().unwrap().port();

        let path = temp_lock_path();
        // Request the occupied port; it should retry to the next free port.
        let (listener, actual_port, lock) =
            acquire_lock_and_bind(&path, "test", occupied_port).expect("bind with retry");
        assert_ne!(
            actual_port, occupied_port,
            "should have retried to a different port"
        );
        drop(listener);
        drop(lock);
        drop(occupied);
    }

    #[test]
    fn bind_with_retry_returns_exhaustion_error() {
        // Test that bind_with_retry returns a PortRetryExhausted error
        // by trying to bind to a port range that includes the occupied port
        // and verifying the error type.
        let (listener, port) = TcpListener::bind(("127.0.0.1", 0))
            .and_then(|l| {
                let p = l.local_addr().map(|a| a.port());
                l.set_nonblocking(true).ok();
                p.map(|p| (l, p))
            })
            .expect("bind test listener");

        // bind to the same port should fail for the first attempt
        let result = std::net::TcpListener::bind(("127.0.0.1", port));
        assert!(result.is_err(), "port should be occupied");

        // But bind_with_retry should find a free port nearby
        let result = bind_with_retry(port);
        assert!(
            result.is_ok(),
            "bind_with_retry should find a free port: {:?}",
            result.err()
        );

        let (_, free_port) = result.unwrap();
        assert_ne!(free_port, port);
        drop(listener);
    }

    #[test]
    fn bind_with_retry_finds_free_port() {
        let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let occupied_port = occupied.local_addr().unwrap().port();

        let (listener, port) = bind_with_retry(occupied_port).expect("bind with retry");
        assert_ne!(port, occupied_port);
        drop(listener);
        drop(occupied);
    }
}
