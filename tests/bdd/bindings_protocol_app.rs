//! protocol-app BDD 绑定：纯协议层场景 + 复用 server 词表的方法表场景。

use crate::tests::bdd::steps_protocol::{ProtocolBdd, protocol_bdd};
use crate::tests::bdd::steps_server::{ServerTest, server_test};
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
    name = "ws-jsonrpc-unary-peer"
)]
async fn test_pa_ws_jsonrpc_unary(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "jsonrpc-envelope-shape"
)]
fn test_pa_env1_jsonrpc_shape(protocol_bdd: ProtocolBdd) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "unary-stable-error-envelope"
)]
async fn test_ip3_stable_error(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "jsonrpc-unary-success-shape"
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
    name = "subscribe-via-jsonrpc"
)]
async fn test_pa_subscribe_jsonrpc(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "approve-tool-is-product-unary"
)]
async fn test_pa_approve_unary(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "jsonrpc-illegal-envelope"
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
