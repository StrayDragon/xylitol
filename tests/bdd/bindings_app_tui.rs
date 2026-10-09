//! BDD bindings for the app-tui-bridge carrier contract (atb4 remote-type-kept).
//! Product attach: HTTP POST /rpc plus HttpWsClient unary on WS /rpc.

use crate::bdd::fixtures::{Workspace, ws};
use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "remote-type-kept"
)]
async fn test_atb4_remote_type_kept(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "no-undriven-skeleton-files"
)]
fn test_at_no_skeleton(ws: Workspace) {}
