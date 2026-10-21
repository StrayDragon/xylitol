use crate::bdd::steps_c2830::{MockUpstreamBdd, mock_upstream_bdd};
use crate::bdd::steps_otel_obs::{OtelBdd, otel_bdd};
use rstest_bdd_macros::scenario;
use serial_test::serial;

#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "厂商原文映射对拍无头"
)]
#[serial]
async fn test_c2830_raw_mapped(otel_bdd: OtelBdd, mock_upstream_bdd: MockUpstreamBdd) {}

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
