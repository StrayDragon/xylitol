use crate::bdd::fixtures::{AgentState, Workspace, agent, ws};
use crate::bdd::steps_cli_surface::{
    AttachBdd, CliEntryBdd, CliHelpBdd, SurfaceBdd, SurfaceFlagsBdd, attach_bdd, cli_entry_bdd,
    cli_help_bdd, surface_bdd, surface_flags_bdd,
};
use crate::bdd::steps_server::{ServerTest, server_test};
use crate::bdd::steps_tokenizer::{TokenizerBdd, tokenizer_bdd};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "tokenizer-help-tree"
)]
fn test_ce15_help(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "tokenizer-status-empty"
)]
fn test_ce15_status_empty(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "tokenizer-download-opt-in"
)]
fn test_ce15_download(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "tokenizer-clean"
)]
fn test_ce15_clean(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "tokenizer-no-bootstrap"
)]
fn test_ce15_no_bootstrap(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "tokenizer-download-shows-hf-base"
)]
fn test_ce15_hf_base(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "surface-tui-verb"
)]
fn test_ce16_tui(surface_bdd: SurfaceBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "surface-print-verb"
)]
fn test_ce16_print(surface_bdd: SurfaceBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "ops-stay-toplevel"
)]
fn test_ce16_ops(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "serve-ops-verb"
)]
fn test_ce8_serve(cli_help_bdd: CliHelpBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "surface-flags-on-tui"
)]
fn test_ce19_tui_flags(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "surface-flags-on-tui-run"
)]
fn test_ce19_tui_run(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "surface-flags-tui-attach"
)]
fn test_ce19_tui_attach(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "surface-flags-on-print"
)]
fn test_ce19_print(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "surface-flags-on-print-trust"
)]
fn test_ce19_print_trust(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "surface-flags-on-print-no-trust"
)]
fn test_ce19_print_no_trust(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "toplevel-surface-flags-rejected"
)]
fn test_ce19_toplevel_reject(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "resume-hint-when-persisted"
)]
fn test_ce20_hint_yes(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "resume-hint-absent-when-unpersisted"
)]
fn test_ce20_hint_no(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "tui-attach-host-down"
)]
fn test_ce21_attach_down(attach_bdd: AttachBdd) {}

// ---- c2826 specs-compact：新增裸规则场景绑定 ----

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "legacy-short-names-not-parsed"
)]
fn test_c2826_legacy_names(cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "builtin-table-product-names"
)]
fn test_c2826_builtin_table(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "idle-slash-parsed-into-pending-commands"
)]
fn test_c2826_idle_slash_pending(cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "unsupported-tree-kind-explicit-error"
)]
async fn test_c2826_tree_kind(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "broken-config-hard-fails"
)]
fn test_c2826_broken_config(ws: Workspace, cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "zero-model-config-hard-fails"
)]
fn test_c2826_zero_models(ws: Workspace, cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "unset-model-shown-as-not-set"
)]
fn test_c2826_unset_display(cli_entry_bdd: CliEntryBdd) {}

#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "cli-is-independent-surface"
)]
fn test_ce_surface_independent(cli_help_bdd: CliHelpBdd) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "app-surface-reaches-agent-via-driver-seam"
)]
fn test_ce_driver_seam() {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "composition-builds-ports-from-infra"
)]
fn test_ce_ports_from_infra(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "cli-and-server-share-one-bootstrap"
)]
fn test_ce_one_bootstrap(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "run-reuses-bootstrap-session-id"
)]
fn test_ce_reuse_session_id(surface_flags_bdd: SurfaceFlagsBdd) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "unpersisted-session-no-resume-hint"
)]
fn test_ce_unpersisted_no_hint(surface_flags_bdd: SurfaceFlagsBdd) {}
#[scenario(
    path = "llmanspec/specs/cli-entry/cli-entry.feature",
    name = "configured-default-shown"
)]
async fn test_ce_c2841_configured_default_shown(agent: AgentState, cli_entry_bdd: CliEntryBdd) {}
