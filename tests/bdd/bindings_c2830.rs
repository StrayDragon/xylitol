//! c2830 native HTTP mock 上游 BDD 绑定（r1462 / r1473 / r1474）。

use crate::tests::bdd::steps_c2830::{MockUpstreamBdd, mock_upstream_bdd};
use crate::tests::bdd::steps_otel_obs::{OtelBdd, otel_bdd};
use rstest_bdd_macros::scenario;
use serial_test::serial;

#[scenario(
    path = "llmanspec/specs/infra-observability/infra-observability.feature",
    name = "provider-raw-mapped-pairing-headless"
)]
#[serial]
async fn test_c2830_raw_mapped(otel_bdd: OtelBdd, mock_upstream_bdd: MockUpstreamBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-generation-input-adapter-capture-headless"
)]
#[serial]
async fn test_c2830_input_capture(otel_bdd: OtelBdd, mock_upstream_bdd: MockUpstreamBdd) {}

#[scenario(
    path = "llmanspec/specs/infra-otel/infra-otel.feature",
    name = "otel-generation-abort-flush-headless"
)]
#[serial]
async fn test_c2830_abort_flush(otel_bdd: OtelBdd, mock_upstream_bdd: MockUpstreamBdd) {}
