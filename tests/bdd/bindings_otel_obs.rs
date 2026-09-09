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
