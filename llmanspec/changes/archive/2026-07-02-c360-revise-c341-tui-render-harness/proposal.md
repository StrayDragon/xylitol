---
change_id: c360-revise-c341-tui-render-harness
title: 修订 c341：解禁 ratatui-widgets 子集 + 落地 TestBackend 渲染测试 harness + 消息部件 enum 骨架
status: proposed
priority: 360
depends_on:
  - c341-drop-umbrella-ratatui-use-core
author: agent
---

# c360-revise-c341-tui-render-harness

## Why

c341 定下的「MUST NOT 用任何 ratatui 内置 widget，全部手写」(spec tui21) 在当时的 MVP 阶段合理（控制依赖、规避 alt-screen 思路的 widget），但三件事让这条约束现在成了**阻碍**：

1. **手写渲染把实现细节绑进了测试**。当前 `commit_to_scrollback`（`src/app/tui/terminal.rs:86-119`）逐 char 写 cell + 手写 CJK filler 逻辑（`buf[(x,y)].set_symbol("")` 占第二列），是全 TUI 最脆弱的代码，却**零测试覆盖**。这正是"写死单测有害"的根源——没有验收机制，改动靠人工肉眼看终端。

2. **手写 CJK 换行屏蔽了更优的现成实现**。`render.rs:162-191` 的 `wrap_to_width` 用 `UnicodeWidthChar::width(ch)` 按单 char 切行，**不处理 emoji ZWJ/grapheme cluster**。而 ratatui 的 `Paragraph::wrap` 已正确处理 CJK 双宽（有日语测试 `ratatui-widgets/src/reflow.rs:544-562`）+ 半角片假名兼容补丁（`ratatui-core/src/buffer/cell_width.rs:50-75`）。c341 的禁令让"复用这个现成实现"成了违规——这违背"能复用就复用"的原则。注意：本变更不是强制用库 widget，而是**解除强制手写的禁令**，让复用成为合法选项；当库不符合预期时自建同样合法。

3. **缺渲染层验收 harness**。当前 30 个 TUI 测试多为逻辑断言（`app.rs:206-262` 喂 XyEvent 断言 Vec 长度），TestBackend 只用于 `draw_tail_frame` 的 cursor 测试（`render.rs:281-288`），**完全没覆盖 insert_before 路径**。没有安全网，就无法安全地调整渲染实现（无论是复用库还是自建）。

4. **渲染层与业务流耦合，没有隔离边界**。`render::commit_lines_for(event: &XyEvent)`（render.rs:119）直接 match 业务事件变体，`app::handle_xy_event`（app.rs:102）把事件处理与渲染行生产混在一起——渲染层知道 `XyEvent` 的存在，业务事件词汇泄漏进 UI。这让 UI/UX 调整会牵动业务逻辑，反之亦然，是脆弱耦合的根源。

### 调研证据

- **ratatui 积木库报告**（`docs/tui-research/ratatui.md` §5.3）：`Paragraph::new(text).wrap(Wrap{trim:false})` 已正确处理 CJK/emoji/零宽；引入 `ratatui-widgets` 只增 ~1 crate，core 类型共享，编译增量小。
- **codex 测试报告**（`docs/tui-research/codex.md` §3）：codex 用 `TestBackend + insta` 做 widget 级快照（36 帧黄金快照），是工业验证的 TUI 测试姿势。
- **三家架构事实**（`docs/tui-research/README.md` §三-bis）：codex/pi/kimi **全是 inline 模式**（无 alt-screen），已完成内容靠 `\r\n` 滚进终端原生 scrollback。xylitol 当前 `Viewport::Inline` 选型与三家一致，是正确方向——本变更是**在 inline 模式内打磨渲染基础设施**，不涉及屏幕模式切换。
- **ratatui 官方测试**（`ratatui/tests/terminal.rs:66`）：`TestBackend + Viewport::Inline + insert_before` 是官方支持的测试组合，有 8 个范例。

### 为什么这几件事捆绑而非拆多个变更

四者互相支撑，分开会产生矛盾：
- **解禁 widgets**（tui21 放宽）让"复用现成实现"成为合法选项——harness 验收的渲染实现可以自由选型（复用或自建），不受一刀切禁令限制。
- **harness**（tui41）是调整渲染实现的安全网——无论是复用库 widget 还是自建，没有 TestBackend 覆盖就等于盲改最脆弱代码（`commit_to_scrollback` 零测试）。
- **边界分离**（tui42）是 harness 能有效验收的前提——只要渲染层还直接 match `XyEvent`，测试就被迫绑定业务事件结构；引入 `RenderedLine` seam 后，harness 可以只测"UI 数据 → 渲染输出"，与业务流解耦。
- **RenderedLine enum** 是边界分离的载体——既是业务→UI 的翻译点，也是 harness 的验证对象。

合成一个变更：先解禁 + 搭 harness 安全网 + 引入 RenderedLine 边界 → 在安全网内按适配度调整渲染实现（复用或自建）→ 用 harness 验收。

## What Changes

1. **修订 spec tui21**（`no-builtin-widgets` → `widget-usage-policy`）：从"MUST NOT 用任何内置 widget"**解禁**为"ratatui-widgets 是允许的依赖，其子集（Paragraph/Block/List/ListState/Clear）能复用就复用；当库 widget 不符合 inline 渲染需求时，MAY 基于 ratatui-core 原语自建独立 widget；选型由适配度驱动，不是一刀切禁止"。**不是退回 umbrella ratatui**——ratatui-widgets 是独立的第三 crate（ratatui-core + ratatui-crossterm 仍直依，tui20 不变）。
2. **新增 spec tui41**（`render-test-coverage`）：TUI 渲染 MUST 有 TestBackend 覆盖，至少覆盖 `commit_to_scrollback`（insert_before 路径）+ `draw_tail_frame` + CJK 换行 + TurnEnd flush。
3. **新增 spec tui42**（`render-business-decoupling`）：渲染层 MUST 与 agent 数据/业务流解耦——渲染函数 MUST 消费 UI 专用数据类型（`RenderedLine` enum），而非直接 match `XyEvent` 变体或调用 Driver/agent 方法。业务事件在单一 seam 处翻译成 UI 数据类型。
4. **Cargo.toml**：在 `tui` feature 下加 `ratatui-widgets = { version = "0.3", default-features = false, optional = true }`（作为可选工具，不是强制依赖）。
5. **建渲染测试 harness**：扩展 `render.rs` 的 test 模块，加 `render_inline` + `commit_and_assert` helper（TestBackend + Viewport::Inline + insert_before），覆盖当前零测试的 commit 路径。
6. **引入 RenderedLine 作为隔离边界**：定义 `RenderedLine { UserInput, AssistantText, ToolSummary, Status }` 作为 UI/UX 层与业务层的 seam——业务流（XyEvent）在此翻译成 UI 数据，渲染层只消费 `RenderedLine`。`commit_to_scrollback` 改为消费 `RenderedLine` 而非 `XyEvent`。
7. **按适配度选型替换手写渲染**：`commit_to_scrollback` 若 `Paragraph::wrap` 能正确处理 CJK 则复用，否则基于 ratatui-core 自建；评估 `draw_tail_frame` 背景填充同理。**不**强求全部用库 widget。

## Capabilities

- `app-tui`（修改）：tui21 放宽 + tui41 新增 + tui42 新增。

## Impact

- **受影响代码**：
  - `Cargo.toml`（+ratatui-widgets 可选依赖）
  - `src/app/tui/render.rs`（harness + RenderedLine 边界 + 按适配度选型）
  - `src/app/tui/terminal.rs`（commit_to_scrollback 选型评估）
  - `src/app/tui/app.rs`（XyEvent→RenderedLine 翻译 seam）
  - `src/app/tui/AGENTS.md`（更新 c341 落地的禁 widget 条款 + 记录边界分离原则）
- **受影响规范**：`app-tui`（tui21 modify + tui41 add + tui42 add）。
- **风险**：中。触及 `commit_to_scrollback`（TUI 最脆弱路径，零测试）+ 引入解耦边界（重构 app.rs 的事件处理）。缓解：先建 harness 再改实现，harness 是安全网。

## 反降级护栏（防止本变更被降级）

- [ ] Cargo.toml 的 umbrella `ratatui` 仍 MUST 缺席；ratatui-widgets MAY 加入但不是强制（选型由适配度驱动）。
- [ ] `commit_to_scrollback` 的手写 cell 循环 MUST 被替换**或**被 TestBackend 测试覆盖（不能继续无测裸跑）。
- [ ] render harness MUST 覆盖至少：空 tail、流式中文换行、turn 结束 flush、CJK filler 行为。
- [ ] 渲染层 MUST NOT 直接 match `XyEvent` 变体或调用 Driver/agent 方法——业务事件 MUST 经 `RenderedLine` seam 翻译（tui42）。
- [ ] 现有 30 个 TUI 测试 + 505 lib + 87 BDD MUST 回归通过。
- [ ] 若用库 widget，`Widget` trait MUST 从 `ratatui_core::widgets::Widget` 引入（不是 `ratatui_widgets::Widget`，后者不存在）。
- [ ] 本变更 MUST NOT 实现 thinking/diff/overlay/对话框（P1-P2 功能，延后）——只做渲染基础设施 + 边界分离。
- [ ] 本变更 MUST NOT 切换到 alt-screen 模式（inline 是三家的正确共识）。
