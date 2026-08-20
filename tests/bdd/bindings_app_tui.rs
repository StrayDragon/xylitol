//! BDD bindings for c2290 TUI attach / HostClient path.

use crate::steps_cli_tokenizer::{AttachBdd, attach_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui/app-tui.feature",
    name = "attach-default"
)]
fn test_tui2_attach_default(attach_bdd: AttachBdd) {}

#[scenario(
    path = "llmanspec/specs/app-tui-bridge/app-tui-bridge.feature",
    name = "remote-type-kept"
)]
fn test_atb4_remote_type_kept() {}
