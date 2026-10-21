use crate::bdd::steps_server::{ServerTest, server_test};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "二进制往返"
)]
async fn test_v3_binary_roundtrip(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "JSON文本RPC拒绝"
)]
async fn test_dual_rail_json_still_served(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "描述宣告格式"
)]
async fn test_describe_declares_formats(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "未知格式致命"
)]
async fn test_unknown_format_fatal(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "工具启动参数原文"
)]
async fn test_v3_tool_start_args_raw(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "代码生成diff干净"
)]
async fn test_codegen_diff_clean(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/protocol-app/protocol-app.feature",
    name = "保留词字段名称保留"
)]
async fn test_reserved_word_field_names_preserved(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "未知方法标识稳定失败"
)]
async fn test_unknown_method_id_stable_fail(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "事件携带序号"
)]
async fn test_v3_event_carries_seq(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "未知变体降级"
)]
async fn test_v3_unknown_variant_degrades(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "产品路径事件等价"
)]
async fn test_dual_rail_event_equivalence(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "产品路径会话快照"
)]
async fn test_dual_rail_session_snapshot_parity(server_test: ServerTest) {}

#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "切换需要对等通过"
)]
async fn test_cutover_requires_parity_green(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "会话树原文载体"
)]
async fn test_c2845_session_tree_raw_parity(server_test: ServerTest) {}
#[scenario(
    path = "llmanspec/specs/server-core/server-core.feature",
    name = "会话树深恢复"
)]
async fn test_c2846_deep_session_tree_parity(server_test: ServerTest) {}
