---
change_id: c398-tui-resize-adaptive
title: TUI 终端实时缩放自适应（字体/窗口尺寸变化下不破坏文字与交互）
status: draft
priority: 398
depends_on: []
author: agent
---

# c398-tui-resize-adaptive

> **状态：draft（草案）**。本变更目前只记录需求与初步技术勘察，不展开 design / tasks / delta specs。
> 待 c396/c397 落地后，结合 c397 引入的「整源 raw_source 重渲」能力再升级为 proposed 并细化方案。

## 需求来源

用户在终端模拟器（如 iTerm2 / Alacritty / Kitty / Windows Terminal / GNOME Terminal）里**实时缩放**：
- Ctrl/Cmd + `+` / `-`：字体变大/变小（改变单元格像素尺寸 → 改变行列数）
- 拖拽窗口边框：改变窗口尺寸（改变行列数）

这些操作都会触发 crossterm 的 `Event::Resize { width, height }`。要求 TUI 能**自调整**：
1. 文字显示不破坏（换行跟随新宽度；CJK 对齐不错位；scrollback 不留残影）。
2. UI 交互显示不破坏（mutable 区 / status 行 / 输入面板布局正确；输入光标位置跟随；输入缓冲内容不丢）。
3. 流式进行中的 turn 不中断、不丢字。

## 现状勘察（代码事实）

### 问题 1：`Event::Resize` 被显式丢弃

`src/app/tui/mod.rs:173-176`：
```rust
Msg::Key(ev) => {
    let Event::Key(key) = ev else {
        continue;   // ← Resize / Mouse / 等都走这里，被忽略
    };
```
阻塞读事件任务（`mod.rs:~126`）把所有 `Event` 包成 `Msg::Key(ev)` 发给主循环，但主循环只解构 `Event::Key`，**其它事件类型（含 Resize）全部 `continue` 丢弃**。结果：终端缩放后，tail 区域的下一帧 `draw_tail` 虽会用 `crossterm::terminal::size()` 拿到新尺寸重画，但缺少**主动的** resize 响应（无重排、无 scrollback 修正），且若期间无新事件触发，界面可能停在旧尺寸直到下一次按键。

### 问题 2：已 commit 的 scrollback 按旧 width 换行，不重排

`render.rs:commit_height` / `TranscriptLine` 在 commit 时用当时的 `width` 计算 CJK 换行；`Terminal::insert_before` **不可逆**（`terminal.rs:64-74`）。resize 后：
- 旧行是按旧宽度换的（如 80 列时 `abcd\nefgh`），新宽度（如 120 列）下本应是一行 `abcdefgh`，但已 commit 行无法重排 → 出现「短行 + 视觉断裂」。
- 变窄（120→80）时，旧行可能超出新宽度被终端硬折行（双行重叠或残影）。

这是 HANDOFF 第三梯队 #7（stable 区可替换）要解决的：codex 的 `AgentMarkdownCell` 在 finalize 时把临时行替换成 source-backed cell，resize 时从源整体重渲。xylitol 当前 commit 不可逆，无此能力。

### 问题 3：mutable region 高度不随尺寸调整

`terminal.rs:27` `TAIL_HEIGHT: u16 = 6` 固定（c397 计划改为动态行数，但仍基于固定预算）。窗口变小（如高度 24→10）时，固定 6 行 tail 可能挤占过多 scrollback 空间；窗口变大也不充分利用。

## 与 c396/c397 的关系

- **c396（样式）**：独立，不影响 resize。可先行落地改善观感。
- **c397（渲染粒度解耦）**：引入「整源 raw_source 重渲」+ `last_full_render` 缓存。**这是 resize 自适应的关键基础设施**——resize 时可对未 commit 的 mutable 行用新 width 整源重渲（c397 已实现每帧全量 render，width 变化天然触发重排）。
- **本变更 c398**：在 c397 基础上补齐 (a) Resize 事件响应接线、(b) scrollback 已 commit 行的重排（需引入 codex 风格 source-backed cell 或接受「旧行不重排」权衡）、(c) tail 高度随窗口高度自适应。

> **建议**：c398 `depends_on: [c397-tui-render-granularity-decouple]`。c397 落地前，c398 只做事件接线（问题 1）即可部分缓解（mutable 区能跟随重画）；完整自适应（问题 2）等 c397 的重渲能力。

## 候选方案（待 proposed 阶段论证）

### 方案 A：仅响应事件 + mutable 重排（最小，接受 scrollback 不重排）
- 接 `Event::Resize` → 标 dirty → 下帧用新 `terminal::size()` 重画 tail。
- mutable 区（c397 后是整源渲染）自动按新 width 重排。
- **scrollback 已 commit 行不重排**（接受断裂，与多数终端 TUI 行为一致——如 less/vim 在 alt-screen 也只重排当前屏）。
- 成本：低。修 mod.rs 接线 + 验证。覆盖需求 1 的 mutable 部分 + 需求 2/3。

### 方案 B：方案 A + scrollback source-backed cell（完整自适应）
- 引入 codex `AgentMarkdownCell` 等价物：finalize 时把流式期的临时 commit 行替换成「source-backed cell」，resize 时从存储的 markdown 源整体重渲整个 scrollback。
- 需 scrollback 支持「替换已 commit 行」（ratatui `insert_before` 不直接支持，需 fork CustomTerminal 或维护自己的 cell store）。
- 成本：高（HANDOFF 第三梯队 #7/#8）。覆盖需求 1 的 scrollback 部分。
- **权衡**：是否值得？终端 TUI 用户对 scrollback 完美重排的期望 vs 实现成本。codex 做了；pi/vim/less 多数不做。

### 方案 C：alt-screen 模式（放弃 inline）
- 切到 `Viewport::Full`（alt-screen），resize 时整屏重画，scrollback 由终端管理。
- **违反 spec tui1**（MUST inline，NOT alt-screen）——拒绝。

**倾向**：方案 A 先行（覆盖 80% 场景），方案 B 作为可选增强记 future.md，方案 C 排除。

## 下一步（升级为 proposed 时）

1. 在 c397 落地后，确认 `last_full_render` 是否已按当前 width 缓存（若缓存了旧 width 的结果，resize 时需失效缓存或带 width key）。
2. 实测：在 iTerm2/Alacritty/Kitty 分别用 Cmd+/- 和拖拽窗口触发 resize，记录 TUI 实际表现（截图），定位需修的具体破坏点。
3. 决定方案 A vs B（用户是否要求 scrollback 完美重排）。
4. 升级为 proposed：补 design.md（方案论证）+ delta specs（modify tui12/tui50，add resize 相关 req）+ tasks.md。
