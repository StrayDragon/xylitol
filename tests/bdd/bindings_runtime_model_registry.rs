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
