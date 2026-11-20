---
depends_on: []
title: app-tui 测试护栏与渲染不变量收厚
needs_specs_change: true
---

## Why

app-tui 的测试护栏集中在两个巨文件里，且渲染不变量没有任何可执行守护：

- `crates/xylitol-app-tui/src/harness.rs` 8695 行，一个文件同时承担三种职责：`ScriptedDriver` 假驱动（`impl XyDriver for ScriptedDriver` 在 525 行，`impl SessionCommandExecutor for ScriptedDriver` 在 1002 行，逐字段镜像 `XyDriver` 全接口面）、终端与按键夹具（`TestTerminal` 1437 行、`enter_event` 等）、以及 `#[cfg(test)] mod slice_tests`（1576 行起）。文件里 345 个函数，其中测试 174 个。
- `crates/xylitol-app-tui/src/tests.rs` 7585 行、194 个测试，是唯一的端到端 harness 场景集合。
- 渲染不变量只靠兜底：`crates/xylitol-app-tui/src/host/mod.rs:1571` `recover_from_render_error` 在 `RenderError` 时静默降级到 TooSmall 模式并只写 `log::warn`。**深挖精确化**：`TooSmall` 模式本身已有测试（`src/tests.rs:66,78,95,111,3743` 断言 `session.mode() == LayoutMode::TooSmall`），但**没有一条**经由 `recover_from_render_error` 触发（`rg 'recover_from_render_error' crates/ tests/ --glob '!target'` 只命中定义 `host/mod.rs:1571` 与两处调用点 `:1144`、`:1191`）。即：由 `RenderError` 驱动的降级路径零覆盖。
- app-tui 只有 2 个 snapshot 文件（`src/snapshots/`，且都是 slash 解析而非整屏），整屏布局 / 换行 / 颜色回归没有基线。**深挖新增**：这两个快照文件本身已漂移且重复——`xylitol__app__tui__commands__parse_tests__slash_bang_parse_surface.snap`（旧名，`source: src/app/tui/commands.rs`）与 `xylitol_app_tui__commands__parse_tests__slash_bang_parse_surface.snap`（新名，`source: crates/xylitol-app-tui/src/commands.rs`）内容在第 111、114 行已不同（`OpenMcp` vs `OpenMcp { panel: Servers }`；`/mcp` usage 文案不同）。旧名文件是历史残留，无测试引用。

后果：改 `host/mod.rs` 或 `layout/` 时，渲染溢出会静默把产品降级成「终端太小」提示，而测试全绿；测试作者为了找一个按键夹具要在 8695 行里翻。这同时压低局部性（护栏知识与变更分散在巨文件）与杠杆（学一个夹具接口就能驱动大量场景的能力被文件规模吃掉）。

## What Changes

- 把 `harness.rs` 按职责拆成三个厚接口模块：假驱动、终端 / 按键夹具、切片测试；保持 `crate::harness` 的公开再导出面不变，调用点不改。
- 为渲染不变量建立可执行守护：`RenderError` 降级路径有测试；整屏渲染在关键宽度下有不变量断言。
- 为固定区建立 snapshot 基线，覆盖产品 UiRoot 的标准档与窄档。
- 清理漂移的旧名快照文件（`xylitol__app__tui__...`），只保留 insta 现行命名的那份。
- 把 `tests.rs` 的巨型场景集合按能力域分组，让新场景有明确落点（不改变既有测试语义）。

## Open Questions

### 已定（2026-11-20 深挖）

- **D0 spec 落点（已确认，用户 2026-11-20）**：渲染溢出降级与宽度边界两条产品可观察规则追加 `app-tui-host`（与 r1270 同域）；snapshot 基线与夹具面稳定属代码组织，不进 spec（spec 约束层级禁止钉代码组织）；撤销新 capability `app-tui-test-guardrails`，`needs_specs_change` 保持 true。

- **降级路径覆盖（已定）**：现有 `TooSmall` 测试不经过 `recover_from_render_error`；须新增一条测试由 `RenderError` 驱动进入降级，并断言 `log::warn` 的 target 为 `xylitol::tui`、文案含 `degraded to TooSmall hint`。
- **旧名快照清理（已定）**：`xylitol__app__tui__commands__parse_tests__slash_bang_parse_surface.snap` 无测试引用且内容已漂移，删除；保留 `xylitol_app_tui__...` 那份。删除前须用 `rg 'slash_bang_parse_surface'` 确认引用点唯一。
- **拆分依据（已定）**：不以行数为由拆（`src/AGENTS.md`「复杂度与体量」明文禁止行数 KPI）。依据是三种调用者接口面不同：假驱动实现 `XyDriver`、夹具实现 `Terminal`、切片测试只调用夹具。

### 待你决定

- **D1 拆分粒度：`harness/` 子目录 vs 平铺文件？**
  - 选项 A（推荐）：`harness/mod.rs` + `scripted.rs` + `terminal.rs` + `samples.rs`。`pub mod harness;` 不变，`crate::harness::*` 调用点零改动。
  - 选项 B：平铺为 `harness_scripted.rs` 等，保持单文件模块。理由：避免目录层级，但公开面需改 `pub use`。

- **D2 `tests.rs`（7585 行 / 194 测试）是否同轮分组？**
  - 选项 A（推荐）：同轮按 host / effects / bridge / layout 域分组，迁入就近模块。理由：新场景有明确落点，但改动面大、需重跑 194 个测试。
  - 选项 B：本轮不动 `tests.rs`，只拆 `harness.rs`。理由：范围更小，风险更低；`tests.rs` 另开 change。

- **D3 渲染不变量是否需要给 `host/mod.rs` 加测试钩子？**
  - 选项 A（推荐）：经 `TestTerminal` 注入超宽帧触发 `RenderError`，**不改产品代码**。
  - 选项 B：给 `host/mod.rs` 加 `#[cfg(test)]` 方法直接调 `recover_from_render_error`。理由：更直接，但侵入产品代码。

## Capabilities

### New Capabilities

无。不新建 capability：harness 模块形状与 snapshot 基线属代码组织 / QA 实践，按 spec 约束层级不进 `.feature`，落点为 design.md 不变量与 tasks.md 判据。

### Modified Capabilities

- `app-tui-host`：追加两条产品可观察规则——渲染溢出 须降级为 TooSmall 提示而非 panic / 损坏帧（扩展 r1270 尺寸降级语义）；任意宽度下渲染结果为成功或降级，不得漏出未处理的 RenderError。既有规则不动。

## Impact

- 代码：`crates/xylitol-app-tui/src/harness.rs`、`crates/xylitol-app-tui/src/tests.rs`、`crates/xylitol-app-tui/src/snapshots/`、`crates/xylitol-app-tui/src/host/mod.rs`（只读，不改行为）。
- 不触碰其他 crate，不触碰 `llmanspec/specs/` 既有 capability 的规则块。
- 门禁：`just test-tui` 与 `just qa` 需全绿；snapshot 新增后需人工复核 `.snap`。

## Further Notes

深挖证据与决策记录见 `design.md`。
