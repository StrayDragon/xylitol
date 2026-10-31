use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReloadEvent {
    pub cwd: std::path::PathBuf,
    pub agent_dir: std::path::PathBuf,
    pub project_trusted: bool,
}

#[async_trait::async_trait]
pub trait ReloadParticipant: Send + Sync {
    fn name(&self) -> &'static str;
    async fn reload(&self, event: &ReloadEvent) -> Result<String, String>;
}

pub struct ReloadHub {
    participants: Mutex<Vec<Arc<dyn ReloadParticipant>>>,
}

impl Default for ReloadHub {
    fn default() -> Self {
        Self::new()
    }
}

impl ReloadHub {
    pub fn new() -> Self {
        Self {
            participants: Mutex::new(Vec::new()),
        }
    }

    pub fn register(&self, participant: Arc<dyn ReloadParticipant>) {
        self.participants
            .lock()
            .expect("reload hub")
            .push(participant);
    }

    pub fn snapshot(&self) -> Vec<Arc<dyn ReloadParticipant>> {
        self.participants.lock().expect("reload hub").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Named(&'static str);

    #[async_trait::async_trait]
    impl ReloadParticipant for Named {
        fn name(&self) -> &'static str {
            self.0
        }

        async fn reload(&self, _event: &ReloadEvent) -> Result<String, String> {
            Ok(self.0.to_string())
        }
    }

    #[test]
    fn snapshot_keeps_registration_order() {
        let hub = ReloadHub::new();
        hub.register(Arc::new(Named("mcp")));
        hub.register(Arc::new(Named("obs")));
        let names: Vec<&str> = hub
            .snapshot()
            .iter()
            .map(|participant| participant.name())
            .collect();
        assert_eq!(names, ["mcp", "obs"]);
    }
}
