//! infra-otel BDD 绑定（otel6/7/8/10/11/12/22 低频观测族）。

use crate::tests::bdd::fixtures::*;
use crate::tests::bdd::fixtures::{Workspace, ws};
use crate::tests::bdd::steps_otel_obs::{OtelBdd, otel_bdd};
use rstest_bdd_macros::scenario;
use serial_test::serial;

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-session-id-on-turn-root-headless"
)]
#[serial]
async fn test_otel6_session_id(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-session-name-metadata-headless"
)]
#[serial]
async fn test_otel7_session_name(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-observation-types-headless"
)]
#[serial]
async fn test_otel8_observation_types(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-turn-span-tree-headless"
)]
#[serial]
async fn test_otel11_span_tree(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-product-span-names-headless"
)]
#[serial]
async fn test_otel12_product_names(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-obs-lane-llm-headless"
)]
#[serial]
async fn test_otel22_obs_lane(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-observation-io-tier-headless"
)]
#[serial]
async fn test_otel10_io_tier(otel_bdd: OtelBdd, agent: AgentState, ws: Workspace) {}

// ---- c2826 specs-compact：裸规则转场景绑定 ----

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-default-none-no-spans"
)]
#[serial]
async fn test_c2826_otel_default_none(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

#[cfg(feature = "otel")]
#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otlp-config-arms-reporter"
)]
#[serial]
async fn test_c2826_otlp_arms(otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otlp-bad-endpoint-falls-back-with-diag"
)]
#[serial]
async fn test_c2826_otlp_bad_endpoint(otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "tool-io-tier-truncated-only-tools"
)]
#[serial]
async fn test_c2826_tool_tier_only(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "compaction-span-over-prepare"
)]
#[serial]
async fn test_c2826_compaction_span(sess: XySessionStore, otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "turn-normal-completion-not-error"
)]
#[serial]
async fn test_c2826_turn_not_error(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "settlement-single-token-estimate"
)]
#[serial]
async fn test_c2826_single_estimate(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}
