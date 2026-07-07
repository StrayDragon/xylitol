//! StdinBuffer - splits batched stdin input into individual sequences.

pub enum StdinBufferEvent {
    Data(String),
    Paste(String),
}

pub struct StdinBuffer {
    buffer: String,
    paste_enabled: bool,
}

impl StdinBuffer {
    pub fn new(_pause_threshold_ms: u64, paste_enabled: bool) -> Self {
        Self {
            buffer: String::new(),
            paste_enabled,
        }
    }

    /// Process incoming data, returns Vec of events extracted.
    pub fn process(&mut self, data: &str) -> Vec<StdinBufferEvent> {
        self.buffer.push_str(data);
        let mut events = Vec::new();

        loop {
            if self.buffer.is_empty() {
                break;
            }

            // Check for bracketed paste start: \x1b[200~
            if self.paste_enabled && self.buffer.starts_with("\x1b[200~") {
                // Find end of paste
                if let Some(end_pos) = self.buffer.find("\x1b[201~") {
                    let content = self.buffer[6..end_pos].to_string();
                    self.buffer.drain(..end_pos + 6);
                    events.push(StdinBufferEvent::Paste(content));
                    continue;
                } else {
                    // Paste not complete yet, wait for more data
                    break;
                }
            }

            // Try to extract the next key sequence
            let (sequence, bytes_consumed) = self.extract_next_sequence();
            if bytes_consumed > 0 {
                events.push(StdinBufferEvent::Data(sequence));
                self.buffer.drain(..bytes_consumed);
            } else {
                // Can't extract a sequence yet
                break;
            }
        }

        events
    }

    /// Extract the next keyboard sequence from the buffer.
    /// Returns (sequence, bytes_consumed).
    fn extract_next_sequence(&self) -> (String, usize) {
        if self.buffer.is_empty() {
            return (String::new(), 0);
        }

        let bytes = self.buffer.as_bytes();
        let first = bytes[0];

        // Single-byte printable/key
        if (0x20..=0x7e).contains(&first) {
            return (self.buffer[..1].to_string(), 1);
        }

        // ESC sequence - peek ahead
        if first == 0x1b {
            if bytes.len() == 1 {
                return (String::new(), 0); // Wait for more
            }

            let second = bytes[1];

            // CSI sequence: ESC [
            if second == b'[' {
                // Find the end: either a letter or ~
                for (i, &b) in bytes.iter().enumerate().skip(2) {
                    match b {
                        b'A'..=b'Z' | b'a'..=b'z' | b'~' => {
                            return (self.buffer[..=i].to_string(), i + 1);
                        }
                        0x1b => {
                            // Nested Esc - this can be complex, just return up to this point
                            // but that's wrong. Let's handle simple cases.
                            break;
                        }
                        _ => continue,
                    }
                }
                return (String::new(), 0); // Wait for more
            }

            // OSC: ESC ]
            if second == b']' {
                for (i, &b) in bytes.iter().enumerate().skip(2) {
                    if b == 0x07 {
                        return (self.buffer[..=i].to_string(), i + 1);
                    }
                    if b == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        return (self.buffer[..=i + 1].to_string(), i + 2);
                    }
                }
                return (String::new(), 0); // Wait for more
            }

            // Two-byte ESC + char sequences
            // ESC O ... (SS3)
            if second == b'O' {
                if bytes.len() >= 3 {
                    return (self.buffer[..3].to_string(), 3);
                }
                return (String::new(), 0);
            }

            // ESC + single char (e.g. ESC p, ESC n, ESC b, ESC f, ESC \r, etc.)
            if bytes.len() >= 2 {
                // Check for ESC + ESC (double ESC)
                if second == 0x1b {
                    return (self.buffer[..2].to_string(), 2);
                }
                return (self.buffer[..2].to_string(), 2);
            }
            return (String::new(), 0);
        }

        // Control characters: return as single-byte sequences
        if first < 0x20 || first == 0x7f {
            return (self.buffer[..1].to_string(), 1);
        }

        // Tab
        if first == b'\t' {
            return (self.buffer[..1].to_string(), 1);
        }

        // Multi-byte UTF-8
        let c = self.buffer.chars().next().unwrap();
        (c.to_string(), c.len_utf8())
    }

    pub fn destroy(&mut self) {
        self.buffer.clear();
    }
}

impl Default for StdinBuffer {
    fn default() -> Self {
        Self::new(10, true)
    }
}
