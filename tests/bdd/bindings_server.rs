use crate::tests::bdd::steps_server::{ServerTest, approval_test, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "start-healthz"
)]
async fn test_server_start_healthz(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "second-instance-rejected"
)]
async fn test_server_second_instance_rejected(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "server-under-app"
)]
async fn test_sr1_server_under_app(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "product-path-jsonrpc"
)]
async fn test_sr_env1_jsonrpc(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "server-rest-ws-under-app"
)]
async fn test_sr2_routes(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "server-lock-under-app"
)]
async fn test_sr3_bind(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "reconnect-replay"
)]
async fn test_sr4_replay(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "approval-roundtrip"
)]
async fn test_sr5_approval(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "lock-file-json"
)]
async fn test_sr7_no_lock(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "port-retry-binds"
)]
async fn test_sr8_no_retry(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "run-endpoint-works"
)]
async fn test_sr9_prompt(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "server-composition-export-io"
)]
fn test_sr10_export(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "no-bare-runtime"
)]
fn test_sr_driver1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "remote-commands"
)]
fn test_sr_remote1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "shared-path"
)]
fn test_sr_dispatch1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "rest-get-tree"
)]
async fn test_sr_st1_rest(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "remote-travel"
)]
fn test_sr_st1_remote(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "remote-session-methods"
)]
fn test_sr_method1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "remote-host-resource-methods"
)]
fn test_sr_resource1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "frame-serialize"
)]
fn test_w1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "subscribe-frame"
)]
async fn test_w2(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "ws-jsonrpc-unary-same-module"
)]
async fn test_w_ws_unary(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "ws-upgrade-mounted"
)]
async fn test_w3(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "seq-monotonic"
)]
fn test_w4(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "resync-on-wrap"
)]
async fn test_w5(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "resync-recover"
)]
async fn test_w6(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "ws-events-pushed"
)]
async fn test_w7(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "cold-restore-snapshot-projection"
)]
async fn test_w8(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "replay-window-does-not-pollute-snapshot"
)]
async fn test_w8_replay_window(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "approve-roundtrip"
)]
async fn test_approval_roundtrip(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "tool-denied"
)]
async fn test_approval_denied(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "approval-broadcast"
)]
async fn test_rr1(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "first-wins"
)]
async fn test_rr2(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "timeout-error"
)]
async fn test_rr3(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "second-answer-ignored"
)]
async fn test_rr4(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "writer-lease"
)]
async fn test_sr_w1_writer_lease(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "writer-token-via-ws-unary"
)]
async fn test_sr_w1_writer_token_ws(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "openapi-debug-doc"
)]
async fn test_sr_oapi1_openapi_debug_doc(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "queue-stats-readonly-unary"
)]
async fn test_sr_q1_queue_stats(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "reload-cooperative-cancel"
)]
async fn test_sr_abort1_cancel(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "abort-idle-falls-back-to-session"
)]
async fn test_sr_abort1_idle(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "subscription-survives-agent-end"
)]
async fn test_sr_sub1_alive(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "idempotent-replay-first-result"
)]
async fn test_sr_idem_replay(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "idempotency-conflict-differs"
)]
async fn test_sr_idem_conflict(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "idempotency-inflight-wait"
)]
async fn test_sr_idem_inflight(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "starting-window-503"
)]
async fn test_sr_rdy1_starting_window(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "ready-flips-healthz"
)]
async fn test_sr_rdy1_ready_flips(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "failed-nonretryable"
)]
async fn test_sr_rdy1_failed(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "serve-writes-registration-file"
)]
async fn test_sr_reg1_writes_file(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "attach-diagnoses-stale-registration"
)]
async fn test_sr_reg1_stale_diagnosis(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "registration-takeover-evicts-old-daemon"
)]
async fn test_sr_reg1_takeover(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "staged-wire-import"
)]
async fn test_sr_imp1_staged_import(server_test: ServerTest) {}
