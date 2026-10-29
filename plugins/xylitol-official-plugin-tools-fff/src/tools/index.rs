use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fff_search::file_picker::{FFFMode, FilePicker, FilePickerOptions};
use fff_search::shared::{SharedFilePicker, SharedFrecency};

pub struct WorkspaceIndex {
    picker: SharedFilePicker,
    partial: bool,
}

impl WorkspaceIndex {
    fn open(base: &Path, watch: bool, scan_timeout: Duration) -> Result<Self, String> {
        let shared = SharedFilePicker::default();
        FilePicker::new_with_shared_state(
            shared.clone(),
            SharedFrecency::default(),
            FilePickerOptions {
                base_path: base.to_string_lossy().into_owned(),
                mode: FFFMode::Ai,
                watch,
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;
        let ready = shared.wait_for_scan(scan_timeout);
        Ok(Self {
            picker: shared,
            partial: !ready,
        })
    }

    pub fn picker(&self) -> &SharedFilePicker {
        &self.picker
    }

    pub fn partial(&self) -> bool {
        self.partial
    }
}

#[derive(Clone)]
pub struct IndexCache {
    inner: Arc<Mutex<HashMap<PathBuf, Arc<WorkspaceIndex>>>>,
    watch: bool,
    scan_timeout: Duration,
}

impl IndexCache {
    pub fn new(watch: bool, scan_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            watch,
            scan_timeout,
        }
    }

    pub fn get(&self, base: &Path) -> Result<Arc<WorkspaceIndex>, String> {
        let key = std::fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
        {
            let guard = self.inner.lock().map_err(|e| e.to_string())?;
            if let Some(index) = guard.get(&key) {
                return Ok(index.clone());
            }
        }
        let index = Arc::new(WorkspaceIndex::open(&key, self.watch, self.scan_timeout)?);
        let mut guard = self.inner.lock().map_err(|e| e.to_string())?;
        Ok(guard.entry(key).or_insert(index).clone())
    }
}
