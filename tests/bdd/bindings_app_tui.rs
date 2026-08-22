//! BDD bindings for the app-tui-bridge carrier contract (atb4 remote-type-kept).

use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "remote-type-kept"
)]
fn test_atb4_remote_type_kept() {}
