use crate::bdd::steps_infra_otel_extra::{MockUpstreamBdd, mock_upstream_bdd};
use crate::bdd::steps_otel_obs::{OtelBdd, otel_bdd};
use rstest_bdd_macros::scenario;
use serial_test::serial;

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "空闲结算独立根"
)]
#[serial]
fn test_ot_idle_root(otel_bdd: OtelBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP生成输入适配器捕获无头"
)]
#[serial]
async fn test_c2830_input_capture(otel_bdd: OtelBdd, mock_upstream_bdd: MockUpstreamBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "OTLP生成中止刷出无头"
)]
#[serial]
async fn test_c2830_abort_flush(otel_bdd: OtelBdd, mock_upstream_bdd: MockUpstreamBdd) {}
