//! layer-architecture BDD：可观察的 server 产品入口（其余分层约束仍为 @human）。

use crate::tests::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/layer-architecture/layer-architecture.feature",
    name = "server-surface-jsonrpc-entry"
)]
async fn test_la_server_jsonrpc_entry(server_test: ServerTest) {}
