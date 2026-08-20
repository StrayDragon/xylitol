use crate::steps_cli_tokenizer::{
    AttachBdd, SurfaceBdd, SurfaceFlagsBdd, TokenizerBdd, attach_bdd, surface_bdd,
    surface_flags_bdd, tokenizer_bdd,
};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-help-tree"
)]
fn test_ce15_help(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-status-empty"
)]
fn test_ce15_status_empty(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-download-opt-in"
)]
fn test_ce15_download(tokenizer_bdd: TokenizerBdd) {}

#[scenario(path = "tests/features/cli-entry.feature", name = "tokenizer-clean")]
fn test_ce15_clean(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-no-bootstrap"
)]
fn test_ce15_no_bootstrap(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tokenizer-download-shows-hf-base"
)]
fn test_ce15_hf_base(tokenizer_bdd: TokenizerBdd) {}

#[scenario(path = "tests/features/cli-entry.feature", name = "surface-tui-verb")]
fn test_ce16_tui(surface_bdd: SurfaceBdd) {}

#[scenario(path = "tests/features/cli-entry.feature", name = "surface-print-verb")]
fn test_ce16_print(surface_bdd: SurfaceBdd) {}

#[scenario(path = "tests/features/cli-entry.feature", name = "ops-stay-toplevel")]
fn test_ce16_ops(tokenizer_bdd: TokenizerBdd) {}

#[scenario(path = "tests/features/cli-entry.feature", name = "serve-ops-verb")]
fn test_ce8_serve(tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "surface-flags-on-tui"
)]
fn test_ce19_tui_flags(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "surface-flags-on-tui-run"
)]
fn test_ce19_tui_run(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "surface-flags-on-print"
)]
fn test_ce19_print(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "surface-flags-on-print-trust"
)]
fn test_ce19_print_trust(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "surface-flags-on-print-no-trust"
)]
fn test_ce19_print_no_trust(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "toplevel-surface-flags-rejected"
)]
fn test_ce19_toplevel_reject(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "resume-hint-when-persisted"
)]
fn test_ce20_hint_yes(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "resume-hint-absent-when-unpersisted"
)]
fn test_ce20_hint_no(surface_flags_bdd: SurfaceFlagsBdd) {}

#[scenario(
    path = "tests/features/cli-entry.feature",
    name = "tui-attach-host-down"
)]
fn test_ce21_attach_down(attach_bdd: AttachBdd) {}
