use crate::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "text-response"
)]
async fn test_agent_text_response(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "tool-call"
)]
async fn test_agent_tool_call(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "turn-order"
)]
async fn test_agent_event_order(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "thinking-switch"
)]
async fn test_agent_thinking_switch(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "thinking-clamp"
)]
async fn test_agent_thinking_limit(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "context-usage"
)]
async fn test_agent_context_usage(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "auto-persist"
)]
async fn test_agent_auto_save(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "no-template-dispatch"
)]
async fn test_sess_no_template_dispatch(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "context-files-found"
)]
fn test_sess_context_files(agent: AgentState, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "second-turn-sees-first"
)]
async fn test_sess_second_turn(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "seed-includes-bash-and-summaries"
)]
fn test_sess_seed(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "persist-load-thinking"
)]
fn test_sess_thinking(agent: AgentState, sess: XySessionStore, ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "legacy-content-rejected"
)]
fn test_sess_legacy(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "turn-events"
)]
async fn test_sess_turn_events(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "tool-stream"
)]
async fn test_sess_tool_stream(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "switch-model"
)]
async fn test_sess_switch_model(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "abort"
)]
async fn test_sess_abort(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "slash-dispatch"
)]
async fn test_sess_slash_dispatch(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "prompt-from-loader"
)]
fn test_sess_prompt_from_loader(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "thinking-toggle"
)]
async fn test_sess_thinking_toggle(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "persist-user-assistant"
)]
async fn test_sess_persist_user_assistant(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "persist-tool-result"
)]
async fn test_sess_persist_tool_result(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "prompt-build"
)]
async fn test_sess_prompt_build(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "product-names-in-get-commands"
)]
fn test_sess_product_names(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "auto-persist-on-message-end"
)]
async fn test_sess_auto_persist_on_end(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "resume-validates-cwd"
)]
async fn test_sess_resume_validates_cwd(agent: AgentState, ws: Workspace, sess: XySessionStore) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "responsibilities-separated"
)]
fn test_sess_responsibilities(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "export-io-injected"
)]
async fn test_sess_export_io(agent: AgentState, sess: XySessionStore, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/agent-session/agent-session.feature",
    name = "no-bash-configured"
)]
async fn test_sess_no_bash(agent: AgentState, ws: Workspace) {}
