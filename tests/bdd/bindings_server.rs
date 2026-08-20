use crate::steps_server::{ApprovalTest, ServerTest, approval_test, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "start-healthz"
)]
fn test_server_start_healthz(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "second-instance-rejected"
)]
fn test_server_second_instance_rejected(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "approve-roundtrip"
)]
fn test_approval_roundtrip(approval_test: ApprovalTest) {}
#[scenario(
    path = "llmanspec/specs/server-core/server-reverse-rpc.feature",
    name = "tool-denied"
)]
fn test_approval_denied(approval_test: ApprovalTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-runtime.feature",
    name = "product-path-four-quadrant"
)]
fn test_sr_env1_four_quadrant(server_test: ServerTest) {}
