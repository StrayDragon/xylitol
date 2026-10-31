use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::XyInProcessDriver;

pub struct ReloadContext {
    pub cwd: std::path::PathBuf,
    pub agent_dir: std::path::PathBuf,
    pub project_trusted: bool,
}

#[async_trait]
pub trait ReloadParticipant: Send + Sync {
    fn name(&self) -> &'static str;
    async fn reload(
        &self,
        driver: &mut XyInProcessDriver,
        ctx: &ReloadContext,
    ) -> Result<String, String>;
}

pub struct ReloadRegistry {
    participants: Mutex<Vec<Arc<dyn ReloadParticipant>>>,
}

impl Default for ReloadRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ReloadRegistry {
    pub fn new() -> Self {
        Self {
            participants: Mutex::new(Vec::new()),
        }
    }

    pub fn register(&self, participant: Arc<dyn ReloadParticipant>) {
        self.participants
            .lock()
            .expect("reload registry")
            .push(participant);
    }

    pub fn snapshot(&self) -> Vec<Arc<dyn ReloadParticipant>> {
        self.participants.lock().expect("reload registry").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Named(&'static str);

    #[async_trait]
    impl ReloadParticipant for Named {
        fn name(&self) -> &'static str {
            self.0
        }

        async fn reload(
            &self,
            _driver: &mut XyInProcessDriver,
            _ctx: &ReloadContext,
        ) -> Result<String, String> {
            Ok(self.0.to_string())
        }
    }

    #[test]
    fn snapshot_keeps_registration_order() {
        let registry = ReloadRegistry::new();
        registry.register(Arc::new(Named("a")));
        registry.register(Arc::new(Named("b")));
        let names: Vec<&str> = registry
            .snapshot()
            .iter()
            .map(|participant| participant.name())
            .collect();
        assert_eq!(names, ["a", "b"]);
    }
}
