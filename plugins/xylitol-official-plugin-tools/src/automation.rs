use std::sync::Arc;

use async_trait::async_trait;
use chrono::Local;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use xylitol_plugin_sdk::TypedTool;
use xylitol_plugin_sdk::error::XyToolError;
use xylitol_plugin_sdk::session::{
    AutomationFact, Cadence, DEFAULT_MAX_TICKS, ScheduleProjection, ScheduleState,
    project_schedules,
};
use xylitol_plugin_sdk::{AutomationGateway, XySessionStore, XyToolCtx};

pub struct SessionAutomationGateway {
    store: Arc<dyn XySessionStore>,
    session_id: RwLock<Option<String>>,
}

impl SessionAutomationGateway {
    pub fn new(store: Arc<dyn XySessionStore>) -> Arc<Self> {
        Arc::new(Self {
            store,
            session_id: RwLock::new(None),
        })
    }

    pub async fn bind_session(&self, session_id: Option<String>) {
        *self.session_id.write().await = session_id;
    }

    async fn active_session_id(&self) -> Result<String, XyToolError> {
        self.session_id
            .read()
            .await
            .clone()
            .ok_or_else(|| XyToolError::ExecutionFailed(anyhow::anyhow!("no active session")))
    }

    async fn load_schedules(
        &self,
        session_id: &str,
    ) -> Result<Vec<ScheduleProjection>, XyToolError> {
        let branch = self
            .store
            .load_leaf_branch(session_id)
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))?;
        Ok(project_schedules(&branch, session_id))
    }

    async fn append(&self, session_id: &str, fact: &AutomationFact) -> Result<(), XyToolError> {
        self.store
            .append_session_entry(session_id, &fact.to_custom_entry())
            .await
            .map_err(|e| XyToolError::ExecutionFailed(anyhow::anyhow!(e)))
    }

    async fn create(&self, args: ScheduleCreateArgs) -> Result<String, XyToolError> {
        let session_id = self.active_session_id().await?;
        if args.prompt.trim().is_empty() {
            return Err(XyToolError::InvalidArgs("prompt must be non-empty".into()));
        }
        args.cadence
            .validate()
            .map_err(|e| XyToolError::InvalidArgs(e.to_string()))?;
        let now = Local::now();
        let schedule_id = uuid::Uuid::new_v4().to_string();
        let max_ticks = args.max_ticks.unwrap_or(DEFAULT_MAX_TICKS);
        self.append(
            &session_id,
            &AutomationFact::ScheduleDefined {
                schedule_id: schedule_id.clone(),
                branch_id: session_id.clone(),
                prompt: args.prompt,
                cadence: args.cadence.clone(),
                max_ticks,
                stop_condition: args.stop_condition,
                created_at: now.timestamp(),
            },
        )
        .await?;
        let next_fire = args.cadence.next_after(now).ok().map(|t| t.timestamp());
        Ok(json!({
            "schedule_id": schedule_id,
            "branch_id": session_id,
            "max_ticks": max_ticks,
            "next_fire": next_fire,
        })
        .to_string())
    }

    async fn list(&self) -> Result<String, XyToolError> {
        let session_id = self.active_session_id().await?;
        let schedules = self.load_schedules(&session_id).await?;
        Ok(serde_json::to_string(&schedules).unwrap_or_else(|_| "[]".into()))
    }

    async fn set_state(
        &self,
        schedule_id: &str,
        state: ScheduleState,
        reason: &str,
    ) -> Result<String, XyToolError> {
        let session_id = self.active_session_id().await?;
        let schedules = self.load_schedules(&session_id).await?;
        if !schedules.iter().any(|s| s.schedule_id == schedule_id) {
            return Err(XyToolError::InvalidArgs(format!(
                "unknown schedule: {schedule_id}"
            )));
        }
        self.append(
            &session_id,
            &AutomationFact::ScheduleStateChanged {
                schedule_id: schedule_id.to_string(),
                state,
                reason: Some(reason.to_string()),
                actor: "llm".into(),
                at: Local::now().timestamp(),
            },
        )
        .await?;
        Ok(json!({ "schedule_id": schedule_id, "state": state }).to_string())
    }
}

#[async_trait]
impl AutomationGateway for SessionAutomationGateway {
    async fn bind_session(&self, session_id: Option<String>) {
        SessionAutomationGateway::bind_session(self, session_id).await;
    }
}

#[derive(Debug, Deserialize)]
pub struct ScheduleCreateArgs {
    pub prompt: String,
    pub cadence: Cadence,
    #[serde(default)]
    pub max_ticks: Option<u64>,
    #[serde(default)]
    pub stop_condition: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleIdArgs {
    pub schedule_id: String,
}

#[derive(Debug, Deserialize)]
pub struct EmptyArgs {}

pub struct ScheduleCreateTool {
    gateway: Arc<SessionAutomationGateway>,
}

impl ScheduleCreateTool {
    pub fn new(gateway: Arc<SessionAutomationGateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl TypedTool for ScheduleCreateTool {
    type Args = ScheduleCreateArgs;

    fn name(&self) -> &str {
        "schedule_create"
    }

    fn description(&self) -> &str {
        "Create a recurring scheduled prompt (interval or 5-field cron) for the current branch."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["prompt", "cadence"],
            "properties": {
                "prompt": { "type": "string", "minLength": 1 },
                "cadence": {
                    "oneOf": [
                        {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["type", "seconds"],
                            "properties": {
                                "type": { "const": "interval" },
                                "seconds": { "type": "integer", "minimum": 1 }
                            }
                        },
                        {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["type", "expr"],
                            "properties": {
                                "type": { "const": "cron" },
                                "expr": { "type": "string" }
                            }
                        }
                    ]
                },
                "max_ticks": { "type": "integer", "minimum": 1 },
                "stop_condition": { "type": "string" }
            }
        })
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        args: ScheduleCreateArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        self.gateway.create(args).await
    }
}

pub struct ScheduleListTool {
    gateway: Arc<SessionAutomationGateway>,
}

impl ScheduleListTool {
    pub fn new(gateway: Arc<SessionAutomationGateway>) -> Self {
        Self { gateway }
    }
}

#[async_trait]
impl TypedTool for ScheduleListTool {
    type Args = EmptyArgs;

    fn name(&self) -> &str {
        "schedule_list"
    }

    fn description(&self) -> &str {
        "List schedules and their projected state for the current branch."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {}
        })
    }

    async fn execute_typed(
        &self,
        ctx: &XyToolCtx,
        _args: EmptyArgs,
    ) -> Result<String, XyToolError> {
        if ctx.cancel.is_cancelled() {
            return Err(XyToolError::Aborted);
        }
        self.gateway.list().await
    }
}

macro_rules! state_tool {
    ($name:ident, $tool:literal, $state:expr, $reason:literal) => {
        pub struct $name {
            gateway: Arc<SessionAutomationGateway>,
        }

        impl $name {
            pub fn new(gateway: Arc<SessionAutomationGateway>) -> Self {
                Self { gateway }
            }
        }

        #[async_trait]
        impl TypedTool for $name {
            type Args = ScheduleIdArgs;

            fn name(&self) -> &str {
                $tool
            }

            fn description(&self) -> &str {
                $reason
            }

            fn parameters_schema(&self) -> Value {
                json!({
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["schedule_id"],
                    "properties": {
                        "schedule_id": { "type": "string" }
                    }
                })
            }

            async fn execute_typed(
                &self,
                ctx: &XyToolCtx,
                args: ScheduleIdArgs,
            ) -> Result<String, XyToolError> {
                if ctx.cancel.is_cancelled() {
                    return Err(XyToolError::Aborted);
                }
                self.gateway
                    .set_state(&args.schedule_id, $state, $reason)
                    .await
            }
        }
    };
}

state_tool!(
    SchedulePauseTool,
    "schedule_pause",
    ScheduleState::Paused,
    "Pause a schedule."
);
state_tool!(
    ScheduleResumeTool,
    "schedule_resume",
    ScheduleState::Active,
    "Resume a paused schedule."
);
state_tool!(
    ScheduleCancelTool,
    "schedule_cancel",
    ScheduleState::Cancelled,
    "Cancel a schedule."
);

pub fn automation_tools(
    gateway: Arc<SessionAutomationGateway>,
) -> Vec<Arc<dyn xylitol_plugin_sdk::XyTool>> {
    vec![
        Arc::new(ScheduleCreateTool::new(gateway.clone())),
        Arc::new(ScheduleListTool::new(gateway.clone())),
        Arc::new(SchedulePauseTool::new(gateway.clone())),
        Arc::new(ScheduleResumeTool::new(gateway.clone())),
        Arc::new(ScheduleCancelTool::new(gateway)),
    ]
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use serde_json::Value;
    use xylitol_plugin_sdk::XySessionStore;
    use xylitol_plugin_sdk::error::{XySessionError, XySessionStoreError};
    use xylitol_plugin_sdk::session::{ForkPosition, SessionContext, SessionEntry};

    use super::*;

    struct MemStore {
        entries: Mutex<Vec<SessionEntry>>,
    }

    impl MemStore {
        fn new() -> Self {
            Self {
                entries: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl XySessionStore for MemStore {
        async fn exists(&self, _session_id: &str) -> bool {
            true
        }

        async fn load_entries(
            &self,
            _session_id: &str,
        ) -> Result<Vec<SessionEntry>, XySessionStoreError> {
            Ok(self.entries.lock().map(|e| e.clone()).unwrap_or_default())
        }

        async fn append_session_entry(
            &self,
            _session_id: &str,
            entry: &SessionEntry,
        ) -> Result<(), XySessionStoreError> {
            self.entries
                .lock()
                .map_err(|_| XySessionStoreError::io("lock", std::io::Error::other("poison")))?
                .push(entry.clone());
            Ok(())
        }

        async fn build_session_context(
            &self,
            _session_id: &str,
        ) -> Result<SessionContext, XySessionStoreError> {
            Ok(SessionContext {
                messages: Vec::new(),
                thinking_level: String::new(),
                model: None,
            })
        }

        async fn create(
            &self,
            _id: &str,
            _cwd: Option<&str>,
            _parent: Option<&str>,
        ) -> Result<(), XySessionStoreError> {
            Ok(())
        }

        async fn fork(
            &self,
            _parent_id: &str,
            _child_id: &str,
            _at_entry_id: &str,
            _position: ForkPosition,
        ) -> Result<(), XySessionError> {
            Err(XySessionError::busy("fork unsupported"))
        }

        fn set_leaf(&self, _session_id: &str, _entry_id: Option<&str>) {}

        fn leaf_id(&self, _session_id: &str) -> Option<String> {
            None
        }
    }

    fn gateway() -> Arc<SessionAutomationGateway> {
        let store: Arc<dyn XySessionStore> = Arc::new(MemStore::new());
        let gateway = SessionAutomationGateway::new(store);
        futures::executor::block_on(gateway.bind_session(Some("s1".into())));
        gateway
    }

    #[test]
    fn tools_cover_schedule_surface() {
        let names: Vec<String> = automation_tools(gateway())
            .iter()
            .map(|tool| tool.name().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "schedule_create",
                "schedule_list",
                "schedule_pause",
                "schedule_resume",
                "schedule_cancel"
            ]
        );
    }

    #[test]
    fn create_list_pause_and_resume() {
        let gateway = gateway();
        let created = futures::executor::block_on(gateway.create(ScheduleCreateArgs {
            prompt: "poll".into(),
            cadence: Cadence::Interval { seconds: 60 },
            max_ticks: Some(3),
            stop_condition: None,
        }))
        .expect("create");
        let created: Value = serde_json::from_str(&created).expect("json");
        let schedule_id = created["schedule_id"].as_str().expect("id").to_string();
        assert_eq!(created["max_ticks"], 3);

        let listed = futures::executor::block_on(gateway.list()).expect("list");
        assert!(listed.contains(&schedule_id));

        futures::executor::block_on(gateway.set_state(
            &schedule_id,
            ScheduleState::Paused,
            "pause",
        ))
        .expect("pause");
        let listed = futures::executor::block_on(gateway.list()).expect("list");
        assert!(listed.contains("paused"));

        futures::executor::block_on(gateway.set_state(
            &schedule_id,
            ScheduleState::Active,
            "resume",
        ))
        .expect("resume");
        let listed = futures::executor::block_on(gateway.list()).expect("list");
        assert!(listed.contains("active"));
    }

    #[test]
    fn invalid_interval_is_rejected() {
        let gateway = gateway();
        let error = futures::executor::block_on(gateway.create(ScheduleCreateArgs {
            prompt: "poll".into(),
            cadence: Cadence::Interval { seconds: 0 },
            max_ticks: None,
            stop_condition: None,
        }))
        .expect_err("invalid");
        assert!(matches!(error, XyToolError::InvalidArgs(_)));
    }

    #[test]
    fn unknown_schedule_is_rejected() {
        let gateway = gateway();
        let error = futures::executor::block_on(gateway.set_state(
            "missing",
            ScheduleState::Paused,
            "pause",
        ))
        .expect_err("missing");
        assert!(matches!(error, XyToolError::InvalidArgs(_)));
    }
}
