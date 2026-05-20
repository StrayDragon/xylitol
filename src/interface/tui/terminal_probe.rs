//! Minimal, bounded terminal probes (Codex-inspired).
//!
//! We only probe for progressive keyboard enhancement support so we can:
//! - Show the right newline hint (`Shift+Enter` vs `Ctrl+J`)
//! - Avoid relying on modifier reporting that some terminals cannot provide.
//!
//! This intentionally uses a short timeout to keep TUI startup snappy.

use std::time::Duration;

/// Detect whether the terminal supports progressive keyboard enhancement (kitty protocol).
///
/// This is best-effort: returns `false` when the probe times out or fails.
pub(crate) fn keyboard_enhancement_supported(timeout: Duration) -> bool {
    #[cfg(unix)]
    {
        imp::keyboard_enhancement_supported(timeout).unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false)
    }
}

#[cfg(unix)]
mod imp {
    use std::fs::File;
    use std::fs::OpenOptions;
    use std::io;
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::os::fd::FromRawFd;
    use std::time::Duration;
    use std::time::Instant;

    use crossterm::event::KeyboardEnhancementFlags;

    pub(super) fn keyboard_enhancement_supported(timeout: Duration) -> io::Result<bool> {
        // Query progressive keyboard enhancement flags + primary device attributes.
        //
        // See <https://sw.kovidgoyal.net/kitty/keyboard-protocol/#detection-of-support-for-this-protocol>
        const QUERY: &[u8] = b"\x1B[?u\x1B[c";

        let mut tty = Tty::open()?;
        tty.write_all(QUERY)?;

        let deadline = Instant::now() + timeout;
        let mut buffer = Vec::<u8>::new();
        let mut saw_flags: Option<KeyboardEnhancementFlags> = None;

        loop {
            tty.read_available(&mut buffer)?;
            let flags = saw_flags.or_else(|| find_keyboard_flags(&buffer));
            if flags.is_some() {
                saw_flags = flags;
            }

            let fallback_seen = find_primary_device_attributes(&buffer).is_some();
            if let Some(flags) = saw_flags
                && fallback_seen
            {
                // We show the `Shift+Enter` hint only when "report all keys" is active,
                // because it is required to reliably disambiguate Shift+Enter from Enter.
                return Ok(
                    flags.contains(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
                );
            }

            if fallback_seen && saw_flags.is_none() {
                return Ok(false);
            }

            let now = Instant::now();
            if now >= deadline {
                return Ok(saw_flags
                    .map(|flags| {
                        flags.contains(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
                    })
                    .unwrap_or(false));
            }

            if !tty.poll_readable(deadline.saturating_duration_since(now))? {
                return Ok(saw_flags
                    .map(|flags| {
                        flags.contains(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
                    })
                    .unwrap_or(false));
            }
        }
    }

    /// Temporary terminal handle used while a startup probe owns terminal input.
    ///
    /// This is adapted from Codex's `terminal_probe.rs`, but reduced to just the keyboard probe.
    struct Tty {
        reader: File,
        writer: File,
        original_flags: libc::c_int,
    }

    impl Tty {
        fn open() -> io::Result<Self> {
            let stdio_reader = dup_file(libc::STDIN_FILENO);
            let stdio_writer = dup_file(libc::STDOUT_FILENO);
            match (stdio_reader, stdio_writer) {
                (Ok(reader), Ok(writer)) => Self::new(reader, writer),
                (reader, writer) => {
                    let stdio_err = match (reader.err(), writer.err()) {
                        (Some(reader_err), Some(writer_err)) => {
                            format!("reader: {reader_err}; writer: {writer_err}")
                        }
                        (Some(reader_err), None) => format!("reader: {reader_err}"),
                        (None, Some(writer_err)) => format!("writer: {writer_err}"),
                        (None, None) => "unknown stdio duplicate error".to_string(),
                    };
                    let reader =
                        OpenOptions::new()
                            .read(true)
                            .open("/dev/tty")
                            .map_err(|fallback_err| {
                                io::Error::new(
                                    fallback_err.kind(),
                                    format!(
                                        "failed to duplicate stdio ({stdio_err}) or open /dev/tty reader ({fallback_err})"
                                    ),
                                )
                            })?;
                    let writer = OpenOptions::new().write(true).open("/dev/tty").map_err(
                        |fallback_err| {
                            io::Error::new(
                                fallback_err.kind(),
                                format!(
                                    "failed to duplicate stdio ({stdio_err}) or open /dev/tty writer ({fallback_err})"
                                ),
                            )
                        },
                    )?;
                    Self::new(reader, writer)
                }
            }
        }

        fn new(reader: File, writer: File) -> io::Result<Self> {
            let fd = reader.as_raw_fd();
            let original_flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if original_flags == -1 {
                return Err(io::Error::last_os_error());
            }
            if unsafe { libc::fcntl(fd, libc::F_SETFL, original_flags | libc::O_NONBLOCK) } == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self {
                reader,
                writer,
                original_flags,
            })
        }

        fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
            self.writer.write_all(bytes)?;
            self.writer.flush()
        }

        fn read_available(&mut self, buffer: &mut Vec<u8>) -> io::Result<()> {
            let mut chunk = [0_u8; 256];
            loop {
                let count = unsafe {
                    libc::read(
                        self.reader.as_raw_fd(),
                        chunk.as_mut_ptr().cast::<libc::c_void>(),
                        chunk.len(),
                    )
                };
                if count > 0 {
                    buffer.extend_from_slice(&chunk[..count as usize]);
                    continue;
                }
                if count == 0 {
                    return Ok(());
                }
                let err = io::Error::last_os_error();
                if matches!(
                    err.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) {
                    return Ok(());
                }
                return Err(err);
            }
        }

        fn poll_readable(&self, timeout: Duration) -> io::Result<bool> {
            let mut fd = libc::pollfd {
                fd: self.reader.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            let deadline = Instant::now() + timeout;
            loop {
                let now = Instant::now();
                if now >= deadline {
                    return Ok(false);
                }
                let timeout_ms = deadline
                    .saturating_duration_since(now)
                    .as_millis()
                    .min(libc::c_int::MAX as u128) as libc::c_int;
                let result = unsafe {
                    libc::poll(&mut fd, /*nfds*/ 1, timeout_ms)
                };
                if result > 0 {
                    return Ok((fd.revents & libc::POLLIN) != 0);
                }
                if result == 0 {
                    return Ok(false);
                }
                let err = io::Error::last_os_error();
                if err.kind() != io::ErrorKind::Interrupted {
                    return Err(err);
                }
            }
        }
    }

    impl Drop for Tty {
        fn drop(&mut self) {
            let _ =
                unsafe { libc::fcntl(self.reader.as_raw_fd(), libc::F_SETFL, self.original_flags) };
        }
    }

    fn dup_file(fd: libc::c_int) -> io::Result<File> {
        let duplicated = unsafe { libc::dup(fd) };
        if duplicated == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_fd(duplicated) })
    }

    fn find_keyboard_flags(buffer: &[u8]) -> Option<KeyboardEnhancementFlags> {
        for start in find_all_subslices(buffer, b"\x1B[?") {
            let rest = &buffer[start + 3..];
            let Some(end) = rest.iter().position(|b| *b == b'u') else {
                continue;
            };
            if end == 0 {
                continue;
            }
            let Ok(bits_text) = std::str::from_utf8(&rest[..end]) else {
                continue;
            };
            let Ok(bits) = bits_text.parse::<u8>() else {
                continue;
            };
            let mut flags = KeyboardEnhancementFlags::empty();
            if bits & 1 != 0 {
                flags |= KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES;
            }
            if bits & 2 != 0 {
                flags |= KeyboardEnhancementFlags::REPORT_EVENT_TYPES;
            }
            if bits & 4 != 0 {
                flags |= KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
            }
            if bits & 8 != 0 {
                flags |= KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
            }
            return Some(flags);
        }
        None
    }

    fn find_primary_device_attributes(buffer: &[u8]) -> Option<()> {
        for start in find_all_subslices(buffer, b"\x1B[?") {
            let rest = &buffer[start + 3..];
            let Some(end) = rest.iter().position(|b| *b == b'c') else {
                continue;
            };
            if end > 0 && rest[..end].iter().all(|b| b.is_ascii_digit() || *b == b';') {
                return Some(());
            }
        }
        None
    }

    fn find_all_subslices<'a>(
        haystack: &'a [u8],
        needle: &'a [u8],
    ) -> impl Iterator<Item = usize> + 'a {
        let mut index = 0;
        std::iter::from_fn(move || {
            let start = haystack[index..]
                .windows(needle.len())
                .position(|window| window == needle)?;
            index += start + 1;
            Some(index - 1)
        })
    }
}
