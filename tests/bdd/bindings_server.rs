use crate::steps_server::{ServerTest, approval_test, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "start-healthz"
)]
async fn test_server_start_healthz(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "second-instance-rejected"
)]
async fn test_server_second_instance_rejected(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "server-under-app"
)]
async fn test_sr1_server_under_app(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "product-path-four-quadrant"
)]
async fn test_sr_env1_four_quadrant(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "server-rest-ws-under-app"
)]
async fn test_sr2_routes(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "server-lock-under-app"
)]
async fn test_sr3_bind(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "reconnect-replay"
)]
async fn test_sr4_replay(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "approval-roundtrip"
)]
async fn test_sr5_approval(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "lock-file-json"
)]
async fn test_sr7_no_lock(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "port-retry-binds"
)]
async fn test_sr8_no_retry(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "run-endpoint-works"
)]
async fn test_sr9_prompt(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "server-composition-export-io"
)]
fn test_sr10_export(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "no-bare-runtime"
)]
fn test_sr_driver1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "remote-commands"
)]
fn test_sr_remote1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "shared-path"
)]
fn test_sr_dispatch1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "rest-get-tree"
)]
async fn test_sr_st1_rest(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "remote-travel"
)]
fn test_sr_st1_remote(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-ws.feature",
    name = "frame-serialize"
)]
fn test_w1(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-ws.feature",
    name = "subscribe-frame"
)]
fn test_w2(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-ws.feature",
    name = "ws-upgrade-mounted"
)]
async fn test_w3(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-ws.feature",
    name = "seq-monotonic"
)]
fn test_w4(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-ws.feature",
    name = "resync-on-wrap"
)]
async fn test_w5(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-ws.feature",
    name = "resync-recover"
)]
async fn test_w6(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-ws.feature",
    name = "ws-events-pushed"
)]
async fn test_w7(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "approve-roundtrip"
)]
async fn test_approval_roundtrip(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "tool-denied"
)]
async fn test_approval_denied(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "approval-broadcast"
)]
async fn test_rr1(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "first-wins"
)]
async fn test_rr2(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "timeout-error"
)]
async fn test_rr3(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "second-answer-ignored"
)]
async fn test_rr4(approval_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "writer-lease"
)]
async fn test_sr_w1_writer_lease(server_test: ServerTest) {}
