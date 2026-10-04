use crate::tests::bdd::fixtures::*;
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "reject-unsupported"
)]
fn test_m10_reject_unsupported(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "reject-case-variant"
)]
fn test_m15_reject_case_variant(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "task-model-resolves-independent"
)]
fn test_m18_task_model_ok(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "task-model-build-failure-falls-back"
)]
fn test_m18_task_model_fallback(agent: AgentState) {}

// ── c2835 后继：runtime-model-registry 裸规则回填 ──────────────────
use crate::tests::bdd::steps_app_tui_host::{HostPumpBdd, host_pump_bdd};
use crate::tests::bdd::steps_runtime_config::{RcSnap, rc_snap};
use crate::tests::bdd::steps_tokenizer::{TokenizerBdd, tokenizer_bdd};

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "provider-registration-drives-round"
)]
async fn test_m20_provider_registration(agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "available-models-surface-in-list"
)]
async fn test_m21_available_models(host_pump_bdd: HostPumpBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "builtin-defaults-two-families-ordered"
)]
fn test_m22_builtin_provider_defaults() {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "resolver-keeps-declared-levels"
)]
fn test_m23_resolver_keeps_levels(rc_snap: RcSnap, tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "resolver-falls-back-to-session-model"
)]
async fn test_m24_task_model_fallback(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "generate-carries-thinking-level"
)]
async fn test_m25_generate_carries_level(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "select-wins-over-settings-default"
)]
fn test_m26_select_wins_over_default(rc_snap: RcSnap) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "entry-api-and-map-survive-resolve"
)]
fn test_m27_entry_api_map(rc_snap: RcSnap, tokenizer_bdd: TokenizerBdd) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "manifest-api-default-by-provider"
)]
fn test_m28_manifest_api_default() {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "explicit-entry-registers-independent-model"
)]
async fn test_m29_explicit_entry_independent(agent: AgentState) {}

#[scenario(
    path = "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature",
    name = "resolver-fallback-keeps-intent"
)]
async fn test_m30_resolver_fallback_intent(agent: AgentState) {}
