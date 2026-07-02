# c360-revise-c341-tui-render-harness — Tasks

> 修订 c341 的 widget 约束（解禁 + 可自建）+ 落地 TUI 渲染测试 harness + RenderedLine 边界 seam。选型由适配度驱动，不预设结论。顺序：先解禁依赖 → 建 harness 安全网 → 引入边界 seam → 按适配度调整渲染实现 → 校验。

## 0. 前置：解禁依赖 + 更新文档

- [x] `Cargo.toml`：在 tui feature 下加 `ratatui-widgets = { version = "0.3", default-features = false, optional = true }`，`tui = [...]` 加 `"dep:ratatui-widgets"`
- [x] `src/app/tui/AGENTS.md`：更新 c341 落地的禁 widget 条款（放宽为「能复用就复用，不符合可基于 ratatui-core 自建」+ 记录 tui42 边界分离原则）
- [x] `cargo build --features tui` 通过（依赖能解析）

## 1. 建渲染测试 harness（安全网，先于实现改动）

在 `src/app/tui/render.rs` 的 `#[cfg(test)]` 模块扩展（复用现有 `render_term` helper，render.rs:284）：

- [x] 加 `render_inline(app, width, height) -> Terminal<TestBackend>` helper（Viewport::Inline）
- [x] 加 `commit_and_assert` helper（走 insert_before + draw + assert_buffer_lines）
- [x] 测试：空 TuiApp 的 tail 基线渲染
- [x] 测试：长中文流式 chunk 的宽度感知换行（固化当前 wrap_to_width 行为）
- [x] 测试：commit 一行中文到 scrollback，断言 CJK 双宽占两列（用 assert_buffer_lines 测行为非实现，块3选型的保护网）
- [x] 测试：TurnEnd 后 tail 无残留（用 harness 重写 tui31 回归，替代逻辑断言）
- [x] 全部测试在【当前手写实现】下绿（先固化行为，再改实现）

## 2. 引入 RenderedLine 边界 seam（tui42）

- [x] 定义 `pub enum RenderedLine<'a> { UserInput, AssistantText, ToolSummary{name,result}, Status }`（放 render.rs 或 app.rs，作为 UI 专用数据类型）
- [x] 加 seam 函数（如 `translate_xyevent(event) -> Vec<RenderedLine>`），集中 XyEvent→RenderedLine 翻译
- [x] `render::commit_lines_for` 改为消费 `&[RenderedLine]` 而非 `&XyEvent`（render.rs:119）
- [x] `app::handle_xy_event` 拆分：业务状态更新留 app，翻译经 seam
- [x] 渲染层（draw_tail_frame / commit_to_scrollback）只接收 RenderedLine/TuiApp，不 match XyEvent
- [x] 不实现 thinking/diff 变体（延后）
- [x] harness 测试覆盖各变体的渲染输出

## 3. 按适配度选型调整渲染实现（复用 OR 自建）

- [x] 评估 `commit_to_scrollback`（terminal.rs:86-119）：试 `Paragraph::new(text).wrap(Wrap{trim:false}).render(buf.area, buf)`，跑块1的 CJK 断言
- [x] 按结果决定：CJK 一致或更优 → 复用 Paragraph 删手写循环；有偏差 → 基于 ratatui-core 自建最小 inline 文本 widget
- [x] 评估 `draw_tail_frame` 背景填充循环（render.rs:39-44）：用 Block/Clear 复用 OR 保留并加测试（按收益决定）
- [x] 评估 `wrap_to_width`（render.rs:162）：若新实现完全覆盖换行，删手写版 + 其 4 个单测；否则保留
- [x] `StatusLine`（render.rs:224）：保持（已足够简单）

## 4. 校验

- [x] `cargo test --features tui --lib tui::` 全过（新增 harness + 既有回归）
- [x] `cargo test --features tui` 全套绿
- [x] `just qa` 绿（fmt + clippy + test + docs；prek 工具本机未安装，非代码问题）
- [x] `llman sdd validate c360-revise-c341-tui-render-harness --strict --no-interactive` 通过（自举：勾选后即过）
- [x] 真终端冒烟：手动跑 `cargo run --features tui`，渲染基础设施（harness/widget/seam）OK；冒烟发现流式布局问题 → 记 future.md 开 c365
