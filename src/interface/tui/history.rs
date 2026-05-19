//! Persistent input history.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Persistent history stored at `~/.xylitol/history`.
#[derive(Debug, Clone)]
pub(crate) struct HistoryStore {
    path: PathBuf,
    max_entries: usize,
    entries: Vec<String>,
}

impl HistoryStore {
    pub(crate) fn load(max_entries: usize) -> io::Result<Self> {
        let Some(home) = dirs::home_dir() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "home directory not found",
            ));
        };
        let path = home.join(".xylitol").join("history");
        Self::load_from(path, max_entries)
    }

    pub(crate) fn load_from(path: PathBuf, max_entries: usize) -> io::Result<Self> {
        let entries = read_lines(&path)?
            .into_iter()
            .filter(|l| !l.is_empty())
            .collect();
        let mut s = Self {
            path,
            max_entries,
            entries,
        };
        s.truncate_to_max();
        Ok(s)
    }

    pub(crate) fn add(&mut self, entry: &str) -> io::Result<()> {
        let entry = entry.trim();
        if entry.is_empty() {
            return Ok(());
        }

        if self.entries.last().is_some_and(|last| last == entry) {
            return Ok(());
        }

        self.entries.push(entry.to_string());
        self.truncate_to_max();
        self.persist()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|s| s.as_str())
    }

    pub(crate) fn search(&self, needle: &str) -> Vec<&str> {
        let needle = needle.trim();
        if needle.is_empty() {
            return Vec::new();
        }
        let needle_lower = needle.to_ascii_lowercase();
        self.entries
            .iter()
            .rev()
            .filter(|s| s.to_ascii_lowercase().contains(&needle_lower))
            .map(|s| s.as_str())
            .collect()
    }

    fn truncate_to_max(&mut self) {
        if self.max_entries == 0 {
            self.entries.clear();
            return;
        }
        if self.entries.len() > self.max_entries {
            let keep = self.max_entries;
            self.entries = self
                .entries
                .split_off(self.entries.len().saturating_sub(keep));
        }
    }

    fn persist(&self) -> io::Result<()> {
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(dir)?;

        let mut data = String::new();
        for line in &self.entries {
            data.push_str(line);
            data.push('\n');
        }
        fs::write(&self.path, data)
    }
}

fn read_lines(path: &Path) -> io::Result<Vec<String>> {
    let data = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    Ok(data.lines().map(|l| l.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_iter_and_search() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history");
        let mut store = HistoryStore::load_from(path.clone(), 3).unwrap();

        store.add("one").unwrap();
        store.add("two").unwrap();
        store.add("three").unwrap();
        store.add("four").unwrap(); // evicts "one"

        let got: Vec<&str> = store.iter().collect();
        assert_eq!(got, vec!["two", "three", "four"]);

        let res = store.search("hr");
        assert_eq!(res, vec!["three"]);

        // Reload from disk.
        let store2 = HistoryStore::load_from(path, 10).unwrap();
        let got2: Vec<&str> = store2.iter().collect();
        assert_eq!(got2, vec!["two", "three", "four"]);
    }
}
