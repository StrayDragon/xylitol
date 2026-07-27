use crate::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "network-domain-block"
)]
fn test_domain_security_network_block() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "deny-write"
)]
fn test_domain_security_deny_write() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "allow-write"
)]
fn test_domain_security_allow_write() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "path-field-bypass"
)]
fn test_ds_path_bypass(ws: Workspace) {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "mcp-default-deny"
)]
fn test_ds_mcp() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "default-enabled"
)]
fn test_ds_default_enabled() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "default-deny-read"
)]
fn test_ds_deny_read() {}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "parent-inheritance"
)]
fn test_ds_parent() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "child-override"
)]
fn test_ds_child() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "override-wins"
)]
fn test_ds_override() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "no-inputs-auto-trust"
)]
fn test_ds_no_inputs() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "fallback-deny-no-ui"
)]
fn test_ds_fallback() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "callback-invoked"
)]
fn test_ds_callback() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "untrusted-blocks-project-settings"
)]
fn test_ds_untrusted_blocks() {}
#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "command-persists"
)]
fn test_ds_cmd_persists() {}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "xy-tool-approval"
)]
fn test_ds_xy_tool_approval(_agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "hook-kill-on-timeout"
)]
async fn test_ds_hook_kill_on_timeout(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "forbidden-pattern-blocks-override"
)]
fn test_ds_forbidden_override() {}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "permission-config"
)]
fn test_ds_permission_config() {}

#[scenario(
    path = "llmanspec/specs/domain-security/domain-security.feature",
    name = "permission-trait"
)]
fn test_ds_permission_trait() {}
