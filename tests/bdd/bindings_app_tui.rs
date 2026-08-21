//! BDD bindings for c2290 TUI attach / HostClient path.

use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "remote-type-kept"
)]
fn test_atb4_remote_type_kept() {}
