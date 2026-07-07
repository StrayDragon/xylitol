/// Ring buffer for Emacs-style kill/yank operations.
pub struct KillRing {
    ring: Vec<String>,
}

impl Default for KillRing {
    fn default() -> Self {
        Self::new()
    }
}

impl KillRing {
    pub fn new() -> Self {
        Self { ring: Vec::new() }
    }

    pub fn push(&mut self, text: String, options: KillRingOptions) {
        if text.is_empty() {
            return;
        }

        if options.accumulate && !self.ring.is_empty() {
            let last = self.ring.pop().unwrap();
            if options.prepend {
                self.ring.push(format!("{}{}", text, last));
            } else {
                self.ring.push(format!("{}{}", last, text));
            }
        } else {
            self.ring.push(text);
        }
    }

    pub fn peek(&self) -> Option<&str> {
        self.ring.last().map(|s| s.as_str())
    }

    /// Move last entry to front (for yank-pop cycling).
    pub fn rotate(&mut self) {
        if self.ring.len() > 1 {
            let last = self.ring.pop().unwrap();
            self.ring.insert(0, last);
        }
    }

    pub fn len(&self) -> usize {
        self.ring.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }
}

pub struct KillRingOptions {
    pub prepend: bool,
    pub accumulate: bool,
}
