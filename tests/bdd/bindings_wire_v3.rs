//! wire v3 场景绑定(c2834;步骤实现见 `steps_wire_v3.rs`)。

use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "v3-binary-roundtrip"
)]
async fn test_v3_binary_roundtrip(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "dual-rail-json-still-served"
)]
async fn test_dual_rail_json_still_served(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "describe-declares-formats"
)]
async fn test_describe_declares_formats(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "unknown-format-fatal"
)]
async fn test_unknown_format_fatal(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "v3-tool-start-args-raw"
)]
async fn test_v3_tool_start_args_raw(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "codegen-diff-clean"
)]
async fn test_codegen_diff_clean(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "reserved-word-field-names-preserved"
)]
async fn test_reserved_word_field_names_preserved(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "unknown-method-id-stable-fail"
)]
async fn test_unknown_method_id_stable_fail(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "v3-event-carries-seq"
)]
async fn test_v3_event_carries_seq(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "v3-unknown-variant-degrades"
)]
async fn test_v3_unknown_variant_degrades(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "dual-rail-event-equivalence"
)]
async fn test_dual_rail_event_equivalence(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "dual-rail-session-snapshot-parity"
)]
async fn test_dual_rail_session_snapshot_parity(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "cutover-requires-parity-green"
)]
async fn test_cutover_requires_parity_green(server_test: ServerTest) {}
