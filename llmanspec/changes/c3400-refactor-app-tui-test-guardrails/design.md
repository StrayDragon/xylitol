# 设计：app-tui 测试护栏与渲染不变量收厚

## 结论

把 app-tui 的测试护栏从「两个巨文件 + 零渲染不变量守护」改成「三个职责清晰的厚夹具模块 + 可执行渲染不变量 + 固定区 snapshot 基线」。触碰面锁在 `xylitol-app-tui` 单 crate 内，不改产品行为，不改其他 crate。

## 现状证据

| 事实 | 证据 |
|---|---|
| harness.rs 8695 行，345 个函数 | `wc -l crates/xylitol-app-tui/src/harness.rs`；`rg -c 'fn ' ` |
| 假驱动逐字段镜像 XyDriver 全接口 | `crates/xylitol-app-tui/src/harness.rs:27` `pub struct ScriptedDriver` 起约 60 个 `Mutex` / `Atomic` 字段；`impl XyDriver for ScriptedDriver` 在 :525；`impl SessionCommandExecutor for ScriptedDriver` 在 :1002 |
| 夹具与测试同居 | `crates/xylitol-app-tui/src/harness.rs:1437` `TestTerminal`；:1521 `key_event`；:1576 `mod slice_tests` |
| tests.rs 7585 行、194 个测试 | `wc -l`；`rg -c '#\[test\]|#\[tokio::test' ` |
| 渲染失败降级无测试 | `crates/xylitol-app-tui/src/host/mod.rs:1571` `recover_from_render_error`；`rg 'recover_from_render_error' crates/ tests/ --glob '!target'` 只命中定义与两处调用点（`:1144`、`:1191`）。`TooSmall` 模式虽有测试（`tests.rs:66,78,95,111,3743`），但**无一由 `RenderError` 驱动** |
| 整屏 snapshot 缺失 | `ls crates/xylitol-app-tui/src/snapshots/` 只有 2 个 `.snap`，都是 `commands::parse_tests` |
| 旧名快照已漂移 | `xylitol__app__tui__commands__parse_tests__slash_bang_parse_surface.snap`（`source: src/app/tui/commands.rs`）与 `xylitol_app_tui__...` 内容在第 111、114 行不同；旧名文件无测试引用 |
| 引擎侧宽度不变量会返回 RenderError | `crates/xylitol-tui/src/tui/render.rs:217` 构造 `RenderError`；`crates/xylitol-tui/src/tui/mod.rs:68` 带 `width` / `line_width` / `line_index` / `line_preview` |

## 删除验证

- **拆 harness.rs 的三块**：想象删掉「夹具与测试同居」这个形状（不是删夹具本身）。夹具与测试同处一个文件不是透传，复杂度不会消失——它会在每个新测试作者找夹具时重新冒出来（8695 行里翻 `enter_event`）。结论：值得加厚为明确接口。
- **渲染不变量守护**：想象删掉降级测试。复杂度不会消失，它会在下一次 `layout/` 改动里以「产品静默降级成 TooSmall 提示、测试全绿」的形式重新冒出来。结论：必须保留并加厚。
- **固定区 snapshot**：想象删掉。整屏布局回归会在每次 `layout/root` 改动里重新冒出来为人工肉眼复核。结论：值得建立基线。

## 与既有约束的关系

- `src/AGENTS.md`「复杂度与体量」明文「不得以文件物理行数作为质量约束」「默认不为行数大拆」。本 change **不以行数为由拆**：拆的依据是职责混居（夹具 / 测试 / 假驱动三种调用者要懂的接口面不同），而不是行数 KPI。
- `crates/xylitol-app-tui/src/AGENTS.md` 角色表已把「harness / tests」列为独立角色，并规定「与生产复杂度门禁混用（测试另计）」。本 change 是把该角色表落到文件结构上。
- 不改 `XyDriver` 接口面，不改 `HostSession`，不改 bridge 缝。`app-tui-host` r1277「host-harness-testable」与 r1278「shared-effect-pump」的既有语义保持。

## 目标形状

```
crates/xylitol-app-tui/src/harness/
  mod.rs          // 公开再导出面：ScriptedDriver / TestTerminal / key_event / enter_event ...
  scripted.rs     // 假驱动：XyDriver + SessionCommandExecutor 实现，脚本与记录
  terminal.rs     // TestTerminal 与按键夹具构造器
  samples.rs      // harness_sample_* 会话树 / 消息样例
```

`crate::harness::*` 的调用点不变；`slice_tests` 移入 `tests.rs` 按能力域分组，或留在 harness 子模块内但独立成文件。最终形态由实施期测量决定，tasks 只钉住「公开面不变」这条硬约束。

```
crates/xylitol-app-tui/src/snapshots/
  <整屏固定区标准档>.snap
  <整屏固定区窄档>.snap
  xylitol_app_tui__...slash_bang_parse_surface.snap   // 保留现行命名那份
  // 删除 xylitol__app__tui__... 旧名漂移副本
```

## 渲染不变量守护点

1. **降级可观测**：构造一个必然触发 `RenderError` 的帧，断言 `recover_from_render_error` 进入 TooSmall 模式且不 panic，并断言 `log::warn` 的 target 为 `xylitol::tui`、文案含 `degraded to TooSmall hint`。

   **优先路径（不改产品代码）**：经 `TestTerminal` 注入超宽帧——`RenderError` 由 `crates/xylitol-tui/src/tui/render.rs:217` 在 `lw > width` 时返回，只要让某行可见宽度超过终端列宽即可触发。若实测不可行，才考虑给 `host/mod.rs` 加 `#[cfg(test)]` 钩子（最小侵入，须在 tasks 单列）。
2. **宽度边界**：对固定区在 `[1, 5, 9, 10, 12, 20, 39, 40, 80, 200]` 列宽下渲染，断言要么 `Ok` 要么进入 TooSmall，不得返回未处理的 `RenderError`。
3. **snapshot 基线**：标准档 80x24 与窄档 40x6 各一张整屏，人工复核后入库。

## 不变量（本 change 的硬约束）

- 既有测试一条都不得删除；迁移后测试名与断言语义不变。
- `crate::harness` 的公开符号集不变（`rg 'crate::harness::|use crate::harness' ` 全部调用点编译通过）。
- 不修改 `crates/xylitol-tui`，不修改产品渲染逻辑。
- 新增 snapshot 不得用 `INSTA_UPDATE=always` 盲接受；必须人工复核后提交。

## 测试边界（seam）

复用既有 `crates/xylitol-app-tui/src/harness.rs` 的 `TestTerminal` + `HostSession` 公共接口，以及 `SceneBuilder`（`crates/xylitol-app-tui/src/interaction_scene.rs`）的无头渲染入口。不新造 seam，不脱离 `.feature` 凭空发明。

## 深挖结论（2026-11-20）

- **降级路径覆盖缺口已确认**：现有 `TooSmall` 测试全部走「终端尺寸直接判定」路径，无一由 `RenderError` 驱动 `recover_from_render_error`。这是真实护栏缺口。
- **旧名快照清理已确认**：`xylitol__app__tui__commands__parse_tests__slash_bang_parse_surface.snap` 无引用且已漂移，删除。
- **拆分依据已确认**：三种调用者接口面不同（假驱动实现 `XyDriver` / 夹具实现 `Terminal` / 切片测试只调夹具），不以行数为由。

## 待确认

- `recover_from_render_error` 的测试是否需为 `host/mod.rs` 加测试可见钩子——见 proposal D3，优先走 `TestTerminal` 注入超宽帧。
- `tests.rs` 的分组粒度——见 proposal D2。
