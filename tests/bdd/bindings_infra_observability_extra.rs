use crate::bdd::fixtures::{AgentState, Workspace, agent, ws};
use crate::bdd::steps_infra_otel_extra::{MockUpstreamBdd, mock_upstream_bdd};
use crate::bdd::steps_otel_obs::{OtelBdd, otel_bdd};
use rstest_bdd_macros::scenario;
use serial_test::serial;

#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "追踪门禁关发出无输出"
)]
#[serial]
async fn test_io_gate_off(agent: AgentState, ws: Workspace, otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "厂商原文映射对拍无头"
)]
#[serial]
async fn test_c2830_raw_mapped(otel_bdd: OtelBdd, mock_upstream_bdd: MockUpstreamBdd) {}
