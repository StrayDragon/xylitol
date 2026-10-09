//! protocol-app BDD 绑定：纯协议层场景 + 复用 server 词表的方法表场景。

use crate::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::bdd::steps_protocol::{ProtocolBdd, protocol_bdd};
use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "command-queue-variants-parse"
)]
fn test_ip_q1_queue_variants(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "command-closed-set-rejects-local-surface"
)]
fn test_pa_cs2_closed_set(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "agent-streaming-events-roundtrip"
)]
fn test_ip7_stream_roundtrip(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "tool-start-args-survive-wire"
)]
fn test_pa_wire2_tool_start_args(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "tool-end-is-error-flag"
)]
fn test_pa_err1_tool_end_flag(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "ws-text-unary-rejected-peer"
)]
async fn test_pa_ws_jsonrpc_unary(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "json-text-envelope-rejected"
)]
async fn test_pa_env1_jsonrpc_shape(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "unary-stable-error-envelope"
)]
async fn test_ip3_stable_error(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "unary-success-shape"
)]
async fn test_pa_jsonrpc_unary_success(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "method-table-unknown-is-32601"
)]
async fn test_pa_method_table_unknown(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "handshake-protocol-via-describe"
)]
async fn test_pa_handshake_describe(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "subscribe-via-rpc"
)]
async fn test_pa_subscribe_jsonrpc(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "approve-tool-is-product-unary"
)]
async fn test_pa_approve_unary(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "illegal-envelope"
)]
async fn test_pa_illegal_envelope(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "queue-stats-method-table-readonly"
)]
async fn test_pa_map4_queue_stats(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "reverse-rpc-first-answer-effective"
)]
async fn test_pa_cs6_first_answer(server_test: ServerTest) {}

// ---- c2826 specs-compact：裸规则转场景绑定 ----

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "queue-update-wire-roundtrip"
)]
fn test_c2826_queue_update_wire(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "agent-part-tagged-content-wire"
)]
fn test_c2826_agent_part_tagged(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "preview-text-excludes-thinking"
)]
fn test_c2826_preview_text(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "error-kind-wire-roundtrip"
)]
fn test_c2826_error_kind_wire(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "todo-updated-wire-roundtrip"
)]
fn test_c2826_todo_updated_wire(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "compaction-end-payload-roundtrip"
)]
fn test_c2826_compaction_end_payload(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "stable-error-kind-at-source"
)]
async fn test_c2826_stable_error_kind(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "switch-session-validates-target"
)]
async fn test_c2826_switch_missing(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "get-messages-returns-entries"
)]
async fn test_c2826_get_messages(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "queue-commands-route-via-unary"
)]
async fn test_c2826_queue_route(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "session-capability-methods-registered"
)]
fn test_c2826_session_methods(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "host-resource-methods-registered"
)]
fn test_c2826_resource_methods(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "estimate-context-method-registered-readonly"
)]
fn test_c2826_estimate_context(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "event-enum-covers-agent-stream"
)]
fn test_pa_event_enum_stream(protocol_bdd: ProtocolBdd) {}
#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "unknown-event-type-is-error-not-panic"
)]
fn test_pa_unknown_event_error(protocol_bdd: ProtocolBdd) {}
#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "no-vendor-specific-event-variants"
)]
fn test_pa_no_vendor_mirror(protocol_bdd: ProtocolBdd) {}
#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "closed-set-rejects-surface-local-command-names"
)]
fn test_pa_closed_set_local(protocol_bdd: ProtocolBdd) {}
#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "no-second-vocabulary-for-surface-locals"
)]
fn test_pa_no_second_vocab(protocol_bdd: ProtocolBdd) {}
#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "host-semantics-on-single-method-table"
)]
fn test_pa_host_single_table(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "bang-runs-outside-agent-tool-stream"
)]
fn test_pa_bang_outside_tool(host_pump_bdd: HostPumpBdd) {}
#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "bash-final-state-in-single-block"
)]
fn test_pa_bash_single_block(host_pump_bdd: HostPumpBdd) {}
