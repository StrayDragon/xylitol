use std::sync::Arc;

use async_trait::async_trait;
use xylitol_plugin_sdk::{BeforeStepCtx, XySessionStore};

use crate::config::{DedupMode, InjectorPoint, Placement};

pub struct InjectorContext<'a> {
    pub before: &'a BeforeStepCtx,
    pub store: Option<&'a dyn XySessionStore>,
    pub session_id: Option<&'a str>,
}

#[async_trait]
pub trait NativeInjector: Send + Sync {
    fn id(&self) -> &str;
    fn point(&self) -> InjectorPoint;
    fn placement(&self) -> Placement;
    fn dedup(&self) -> DedupMode;
    fn group(&self) -> Option<&str>;
    fn priority(&self) -> i32 {
        0
    }
    async fn produce(&self, ctx: &InjectorContext<'_>) -> Option<String>;
}

pub enum InjectorSource {
    Command { command: String, timeout_secs: u64 },
    Native(Arc<dyn NativeInjector>),
}

pub struct ContextInjector {
    pub id: String,
    pub point: InjectorPoint,
    pub priority: i32,
    pub placement: Placement,
    pub dedup: DedupMode,
    pub group: Option<String>,
    pub source: InjectorSource,
}

pub struct ContextInjectorRegistry {
    injectors: std::sync::RwLock<Vec<Arc<ContextInjector>>>,
}

impl ContextInjectorRegistry {
    pub fn new() -> Self {
        Self {
            injectors: std::sync::RwLock::new(Vec::new()),
        }
    }

    pub fn register(&self, injector: ContextInjector) -> Result<(), String> {
        let mut guard = self
            .injectors
            .write()
            .map_err(|_| "context injector registry poisoned".to_string())?;
        if guard.iter().any(|existing| existing.id == injector.id) {
            return Err(format!("duplicate context injector id: {}", injector.id));
        }
        guard.push(Arc::new(injector));
        Ok(())
    }

    pub fn snapshot(&self) -> Vec<Arc<ContextInjector>> {
        let mut injectors = self
            .injectors
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        injectors.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
        injectors
    }

    pub fn is_empty(&self) -> bool {
        self.injectors
            .read()
            .map(|guard| guard.is_empty())
            .unwrap_or(true)
    }
}

impl Default for ContextInjectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DedupMode, InjectorPoint, Placement};

    fn command_injector(id: &str, priority: i32) -> ContextInjector {
        ContextInjector {
            id: id.into(),
            point: InjectorPoint::TurnPre,
            priority,
            placement: Placement::Ephemeral,
            dedup: DedupMode::None,
            group: None,
            source: InjectorSource::Command {
                command: "true".into(),
                timeout_secs: 2,
            },
        }
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let registry = ContextInjectorRegistry::new();
        registry.register(command_injector("a", 0)).expect("first");
        assert!(registry.register(command_injector("a", 1)).is_err());
    }

    #[test]
    fn snapshot_orders_by_priority_then_id() {
        let registry = ContextInjectorRegistry::new();
        registry.register(command_injector("b", 1)).expect("b");
        registry.register(command_injector("a", 1)).expect("a");
        registry.register(command_injector("z", 0)).expect("z");
        let ids: Vec<_> = registry
            .snapshot()
            .into_iter()
            .map(|i| i.id.clone())
            .collect();
        assert_eq!(ids, vec!["z", "a", "b"]);
    }
}
