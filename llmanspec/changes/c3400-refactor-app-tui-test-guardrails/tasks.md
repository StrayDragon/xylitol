# 任务：app-tui 测试护栏与渲染不变量收厚

> 全部任务在 change 绑定分支上执行。完成判据中涉及计数 / 基线的项，在 change 分支上相对 merge-base 测量。

## T1 拆出假驱动模块（公开面不变）

- [ ] 把 `ScriptedDriver`（结构体 + `impl XyDriver` + `impl SessionCommandExecutor` + 脚本与记录方法）从 `crates/xylitol-app-tui/src/harness.rs` 迁入 `crates/xylitol-app-tui/src/harness/scripted.rs`。
- [ ] `harness/mod.rs` 保持 `pub use scripted::ScriptedDriver;`，公开符号集不变。
- [ ] 判据：`rg 'crate::harness::ScriptedDriver|use crate::harness::\{' ` 的全部调用点编译通过；`cargo nextest run -p xylitol-app-tui` 全绿。

## T2 拆出终端与按键夹具模块

- [ ] 把 `TestTerminal`（`impl Terminal`）、`key_event` / `enter_event` / `alt_enter_event` / `esc_event` 等按键构造器迁入 `crates/xylitol-app-tui/src/harness/terminal.rs`。
- [ ] 把 `harness_sample_message_history_tree` / `harness_sample_session_messages` / `flatten_session_tree_entries` 迁入 `crates/xylitol-app-tui/src/harness/samples.rs`。
- [ ] 判据：公开符号集不变；`cargo nextest run -p xylitol-app-tui` 全绿。

## T3 拆出切片测试模块

- [ ] 把 `#[cfg(test)] mod slice_tests`（`harness.rs:1576` 起）迁入独立文件，按能力域分组。
- [ ] 判据：迁移前后测试数量一致（`cargo nextest list -p xylitol-app-tui | wc -l` 前后差值仅来自新增测试）；测试名与断言语义不变；全绿。

## T4 渲染失败降级可观测

- [ ] 构造必然触发 `RenderError` 的帧：经 `TestTerminal` 注入可见宽度超过终端列宽的内容（`crates/xylitol-tui/src/tui/render.rs:217` 在 `lw > width` 时返回 `RenderError`）。**优先不改产品代码**；若实测不可行，才给 `host/mod.rs` 加 `#[cfg(test)]` 钩子并在本 task 记录理由。
- [ ] 断言：进入 `LayoutMode::TooSmall`、不 panic、`log::warn` target 为 `xylitol::tui`、文案含 `degraded to TooSmall hint`。
- [ ] 判据：该测试在 `recover_from_render_error` 被移除时会失败（反向验证）；既有 `tests.rs:66,78,95,111,3743` 的 `TooSmall` 断言保持绿。

## T4b 清理漂移的旧名快照

- [ ] 先确认引用：`rg 'slash_bang_parse_surface' crates/ tests/ --glob '!target'`，确认无代码按旧名引用。
- [ ] 删除 `crates/xylitol-app-tui/src/snapshots/xylitol__app__tui__commands__parse_tests__slash_bang_parse_surface.snap`（`source` 指向旧路径 `src/app/tui/commands.rs`，内容已与现行命名那份漂移）。
- [ ] 保留 `xylitol_app_tui__commands__parse_tests__slash_bang_parse_surface.snap`。
- 判据：`cargo nextest run -p xylitol-app-tui` 全绿；`ls crates/xylitol-app-tui/src/snapshots/` 不再含旧名文件。

## T5 宽度边界不变量

- [ ] 对固定区在 `[1, 5, 9, 10, 12, 20, 39, 40, 80, 200]` 列宽下渲染，断言结果为 `Ok` 或进入 TooSmall，不得返回未处理的 `RenderError`。
- [ ] 判据：新增测试全绿；覆盖 40 列标准档下限与 6 行高度下限两侧。

## T6 固定区整屏 snapshot 基线

- [ ] 新增标准档 80x24 与窄档 40x6 两张整屏 `.snap`，落在 `crates/xylitol-app-tui/src/snapshots/`。
- [ ] 判据：人工复核 `.snap` 内容后提交；`INSTA_UPDATE=always` 不用于首次接受。

## T7 `tests.rs` 按能力域分组

- [ ] 把 `crates/xylitol-app-tui/src/tests.rs` 的 194 个测试按 host / effects / bridge / layout 域分组，或迁入就近模块的 `tests.rs`。
- [ ] 判据：测试数量与名称不变；全绿。

## T9 落地 specs（绑定分支后执行）

- [ ] 在绑定分支向 `llmanspec/specs/app-tui-host/app-tui-host.feature` 追加两条规则（渲染溢出降级为 TooSmall 提示而非 panic / 损坏帧；任意宽度渲染结果为成功或降级，不得漏出未处理的 RenderError），各带嵌套 `场景:`（无头 harness 驱动）。
- [ ] 在 `tests/bdd/` 补对应 scenario 绑定（bindings_app_tui_host.rs 与 steps）。
- [ ] 判据：`llman-sdd validate app-tui-host` 通过；`cargo nextest run --all-features --test bdd` 绿。

## T8 全量门禁

- [ ] `just fmt`、`just lint`、`cargo nextest run -p xylitol-app-tui`、`just qa` 全绿。
- [ ] 判据：`just qa` 输出无失败；`llman-sdd validate c3400-refactor-app-tui-test-guardrails` 无 ERROR。
