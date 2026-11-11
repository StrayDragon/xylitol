use rstest_bdd_macros::scenario;

#[scenario(
    path = "llmanspec/specs/app-tui-trust/app-tui-trust.feature",
    name = "信任选择主题与取消"
)]
fn test_atq_theme_cancel() {}

#[scenario(
    path = "llmanspec/specs/app-tui-trust/app-tui-trust.feature",
    name = "信任斜杠持久化无自动重载"
)]
fn test_atq_slash_persist() {}

#[scenario(
    path = "llmanspec/specs/app-tui-trust/app-tui-trust.feature",
    name = "信任门禁垃圾答案取消"
)]
fn test_atq_garbage_answer() {}
