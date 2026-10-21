use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::ports::{XyModel, XyTool};
use crate::resource::SkillInfo;

pub struct Registry<T: ?Sized + Send + Sync + 'static> {
    inner: Mutex<HashMap<String, Arc<T>>>,
}

impl<T: ?Sized + Send + Sync + 'static> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized + Send + Sync + 'static> Registry<T> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, name: impl Into<String>, value: Arc<T>) {
        self.inner
            .lock()
            .expect("registry")
            .insert(name.into(), value);
    }

    pub fn remove(&self, name: &str) -> Option<Arc<T>> {
        self.inner.lock().expect("registry").remove(name)
    }

    pub fn get(&self, name: &str) -> Option<Arc<T>> {
        self.inner.lock().expect("registry").get(name).cloned()
    }

    pub fn snapshot(&self) -> Vec<(String, Arc<T>)> {
        self.inner
            .lock()
            .expect("registry")
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().expect("registry").is_empty()
    }
}

pub struct HostRegistries {
    pub models: Registry<dyn XyModel>,
    pub tools: Registry<dyn XyTool>,
    pub skills: Registry<SkillEntry>,
}

pub struct SkillEntry {
    pub info: SkillInfo,
}

impl Default for HostRegistries {
    fn default() -> Self {
        Self::new()
    }
}

impl HostRegistries {
    pub fn new() -> Self {
        Self {
            models: Registry::new(),
            tools: Registry::new(),
            skills: Registry::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_get_remove() {
        let reg = Registry::<u32>::new();
        reg.insert("a", Arc::new(1));
        assert_eq!(*reg.get("a").unwrap(), 1);
        assert!(reg.remove("a").is_some());
        assert!(reg.is_empty());
    }
}
