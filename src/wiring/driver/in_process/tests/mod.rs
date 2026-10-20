// bind_session writes obs via ObsSessionScope — no process-slot race with CollectingReporter tests.
use std::sync::Arc;
use xylitol_official_plugin_provider_openai::provider::{ObsSessionContext, ObsSessionScope};

use super::*;
use crate::wiring::infra::event::EventBus;
use crate::wiring::infra::permission;
use xylitol_official_plugin_session::SessionManager;
use xylitol_plugin_sdk::session::{EntryBase, MessageEntry, SessionEntry};
use xylitol_plugin_sdk::{XyEventSink, XySessionStore};
use xylitol_runtime::agent::AgentBuilder;
use xylitol_runtime::agent::tools::ToolSet;

type ModelBuilderFn = xylitol_plugin_sdk::XyModelBuilder;

fn msg_entry(id: &str, parent: Option<&str>, role: &str, text: &str) -> SessionEntry {
    SessionEntry::Message(MessageEntry {
        base: EntryBase {
            entry_type: "message".into(),
            id: id.into(),
            parent_id: parent.map(str::to_string),
            timestamp: entry_ms(id),
        },
        message: xylitol_plugin_sdk::session::fixture_message_json(role, text),
    })
}

/// Deterministic unix-ms from a string id (test fixture).
fn entry_ms(id: &str) -> u64 {
    1_781_827_200_000u64
        + id.as_bytes()
            .iter()
            .fold(0u64, |acc, b| acc * 31 + *b as u64)
            % 1_000_000
}

async fn build_test_driver(store: Arc<SessionManager>) -> (XyInProcessDriver, ObsSessionScope) {
    let scope = ObsSessionScope::enter(ObsSessionContext::default());
    let store_trait: Arc<dyn XySessionStore> = store.clone();
    let mut agent = AgentBuilder::new(
        xylitol_runtime::agent::model::registry::ModelRegistry::new(),
        Arc::new(crate::wiring::infra::provider::factory::build_provider),
        store_trait.clone(),
        Arc::new(EventBus::new()) as Arc<dyn XyEventSink>,
        permission::allow_all_permission(),
    )
    .cwd(".")
    .tools(ToolSet::from_iter(
        xylitol_official_plugin_tools::default_tools(),
    ))
    .build_baseline()
    .materialize_runtime();
    let sid = uuid::Uuid::new_v4().to_string();
    store_trait
        .create(&sid, Some("."), None)
        .await
        .expect("create session");
    agent.bind_session(sid).expect("bind_session");
    (XyInProcessDriver::new(agent, store), scope)
}

mod bash;
mod mcp;
mod session;
mod trust;
