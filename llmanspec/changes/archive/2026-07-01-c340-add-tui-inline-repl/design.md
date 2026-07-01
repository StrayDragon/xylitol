# c340 Design — TUI inline REPL 落地

本变更依据 c325 的架构决策，把 TUI 实体化为可运行代码。设计目标：**把风险压到与已验证的 print 模式同级**。

## 1. 已核实的 API 地基（防止虚构）

实施前已从 vendored 源码确认 ratatui 0.30.2 的精确签名（`ratatui-core-0.1.2`）：

| 用途 | API | 源码位置 |
|---|---|---|
| 创建 inline 终端 | `ratatui::try_init_with_options(&TerminalOptions { viewport: Viewport::Inline(N) }) -> Result<DefaultTerminal>` | `ratatui-0.30.2/src/lib.rs:499`（re-export）, `init.rs:65` |
| 退出恢复 | `ratatui::restore()` | `lib.rs:499` |
| 重绘可变尾部 | `terminal.draw(\|frame\| { ... }) -> Result<CompletedFrame>` | `terminal.rs` |
| 提交到 scrollback | `terminal.insert_before(height: u16, draw_fn: impl FnOnce(&mut Buffer)) -> Result<()>` | `inline.rs:109` |
| 异步键盘 | `crossterm::event::EventStream`（需 `event-stream` feature）实现 `Stream` | `crossterm-0.29.0/src/event.rs:124` |

`Viewport::Inline(u16)` 的 u16 = live tail 行数（`viewport.rs:99`）。Inline「span the full terminal width」，自动 resize（`init.rs:83`）。

## 2. 模块设计

### 2.1 `terminal.rs` — 终端生命周期（RAII）

```rust
pub struct InlineTerminal {
    term: ratatui::DefaultTerminal,
}
impl InlineTerminal {
    /// 进入 raw mode + Inline viewport（不进 alt screen）。
    pub fn enter(tail_height: u16) -> std::io::Result<Self> {
        enable_raw_mode()?;
        let term = ratatui::try_init_with_options(&ratatui::TerminalOptions {
            viewport: ratatui::Viewport::Inline(tail_height),
        })?;
        Ok(Self { term })
    }
    /// 重绘尾部 N 行（spinner/工具状态/当前流式行）。
    pub fn draw_tail(&mut self, app: &TuiApp) -> std::io::Result<()> {
        self.term.draw(|frame| { render::draw_tail_frame(frame, app); })?;
        Ok(())
    }
    /// 把已完成的行提交到 scrollback（之后不再触碰）。
    pub fn commit_to_scrollback(&mut self, lines: &[ratatui::text::Line]) -> std::io::Result<()> {
        let height = lines.len() as u16;
        self.term.insert_before(height, |buf| {
            let area = buf.area;
            for (i, line) in lines.iter().enumerate() {
                if i < area.height as usize {
                    line.render(area.row(i as u16), buf);  // 用 ratatui text::Line 渲染
                }
            }
        })?;
        Ok(())
    }
    pub fn flush_resize(&mut self) { let _ = self.term.draw(|_| {}); }
}
impl Drop for InlineTerminal {
    fn drop(&mut self) {
        ratatui::restore();
        let _ = disable_raw_mode();
    }
}
```
**关键**：`Drop` 保证 raw mode 关闭——即使 panic 也不留坏终端（spec tui15）。

### 2.2 `mod.rs` — REPL 主循环

```rust
pub async fn run(driver: &mut dyn Driver, /* 装配好的 agent 上下文 */) -> Result<(), String> {
    let mut term = InlineTerminal::enter(TAIL_HEIGHT)?;
    let mut app = TuiApp::default();
    let mut input = crossterm::event::EventStream::new();
    let cancel = CancellationToken::new();

    loop {
        tokio::select! {
            // 源 1: 当前 turn 的 XyEvent 流（仅在有活跃 stream 时轮询）
            Some(ev) = async { app.event_rx.recv().await }, if app.is_streaming => {
                let committed = app.handle_xy_event(ev);
                if let Some(lines) = committed { term.commit_to_scrollback(&lines)?; }
                term.draw_tail(&app)?;
                if app.turn_done() { app.end_stream(); }
            }
            // 源 2: 键盘
            maybe_key = input.next() => {
                if let Some(Ok(ev)) = maybe_key {
                    match input::handle(ev, &mut app)? {
                        InputOutcome::Submit(prompt) => {
                            let stream = driver.run(&prompt).await;
                            app.start_stream(stream);
                        }
                        InputOutcome::Slash(cmd) => commands::dispatch(cmd, driver, &mut app).await?,
                        InputOutcome::Abort => { driver.abort(); cancel.cancel(); break; }
                        InputOutcome::Quit => break,
                        InputOutcome::Idle => {}
                    }
                    term.draw_tail(&app)?;
                }
            }
            // 源 3: 取消
            _ = cancel.cancelled(), if app.is_streaming => {
                driver.abort(); app.end_stream();
            }
        }
    }
    Ok(())
}
```
**设计要点**：XyEvent 流不在主 select! 里直接 `.next()`，而是由 `app.start_stream` spawn 一个转发任务把 stream drain 到 `event_rx`（mpsc）。原因：`Driver::run` 返回的 `EventStream`（`Pin<Box<dyn Stream>>`）不能直接放进 `select!` 的 future（借用问题），用 mpsc 解耦更干净。

### 2.3 `render.rs` — XyEvent → 渲染（纯函数优先）

- `draw_tail_frame(frame, app)`：把 app 当前态（输入缓冲、spinner、当前流式行、最近工具状态）画进 frame 的 area。纯呈现，无副作用。
- `xyevent_to_committed(ev) -> Option<Vec<Line>>`：决定哪些事件产出「提交到 scrollback 的完整行」。TextDelta 累积，遇 `\n` 边界产出完整行；ToolExecutionEnd 产出一个摘要行；TurnEnd 提交剩余。**纯函数，可单测**（喂确定性 XyEvent，断言产出行）。

### 2.4 `input.rs` — 键位

MVP 单行输入：打印字符追加缓冲，Backspace 删除，Enter 提交，Ctrl+C abort，识别 `/` 开头为 slash。多行编辑（tui-textarea 等组件）**推迟**到后续变更（按 write-surface「不为暂时没用加骨架」）。

### 2.5 `commands.rs` — slash 命令

`/exit` → 退出循环；`/model` → 调 driver/agent 切模型（复用 `protocol::Command::SetModel/CycleModel` 语义）。完整共享 dispatch 是 c335 的职责；MVP 本地最小分派。

### 2.6 `theme.rs` — 语义 token

对标 kimi-code `colors.ts`：定义 `primary`/`text`/`text_dim`/`success`/`error` 等 token，映射到 ratatui `Color`。组件不得硬编码颜色字面量。

## 3. Mode 分发接入（`cli/mod.rs`）

对齐 pi `resolveAppMode`（`main.ts:100-111`）：
```rust
// 现有: 有 prompt → print; --rpc → rpc
// 新增: 无 prompt 且 stdin 是 TTY → tui
let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdin());
if args.prompt.is_none() && is_tty && !args.rpc {
    return crate::app::tui::run(/* 装配好的 driver + ctx */).await.map_err(...);
}
```
**装配复用**：TUI 复用 cli/mod.rs 既有的 config→registry→trust→resource→build_agent 全流程（c330 的共享抽取未 full 化前，临时直接调用既有函数/最小复制；c330 full 化后零成本切换）。

## 4. 风险与缓解（提前识别）

### 风险 R1：EventStream borrow / select! 整合困难【中】
`Driver::run` 返回 `Pin<Box<dyn Stream + Send>>`，直接在 `select!` 里持有可变借用易出 borrow check 错误。
**缓解**：spawn 转发任务 drain stream → mpsc channel，主循环只 `event_rx.recv()`（如 2.2 所述）。这是 codex 的同类模式（AppEvent channel），已验证可行。

### 风险 R2：insert_before 在高频 TextDelta 下闪烁【低-中】
无 `scrolling-regions` feature 时，insert_before 用 clear-and-redraw fallback。
**缓解**：MVP 先按「行边界批量提交」（TextDelta 累积到 `\n` 才 commit），降低 insert 频率。若仍闪烁，后续独立变更加 `scrolling-regions` feature（一行 Cargo 改动）。**不在 MVP 引入**（避免不稳定 feature）。

### 风险 R3：panic 留下坏终端【中】
ratatui panic 在 raw mode 下会搞乱用户终端。
**缓解**：`InlineTerminal::Drop` 调 `restore()` + `disable_raw_mode()`。另用 `std::panic::set_hook` 或把主循环包在 catch_unwind 里确保 Drop 运行（codex 的 `tui.rs` 正是这套）。

### 风险 R4：装配逻辑复制（c330 未 full 化）【低】
MVP 直接调用 cli/mod.rs 既有装配，可能产生小量复制。
**缓解**：c340 内「最小调用」而非「完整复制」——优先调用既有 pub 函数；c330 full 化时统一收拢。反降级护栏确保 c340 不夹带完整 bootstrap 重构。

### 风险 R5：arch_guard 误报【低】
TUI import `crate::agent`（mod 级）可能触发 arch_guard 对 agent 内部的检查。
**缓解**：write-surface 明确允许面 import `crate::agent` mod 级 + `app::core`；arch_guard 现有 exemption 已含 `tui/`。提交前 `just qa` 验证。

### 风险 R6：TTY 检测误判（CI/管道环境）【低】
`IsTerminal` 在某些环境行为差异。
**缓解**：`--tui` 显式 flag 作为强制入口；默认检测只增不损（非 TTY 仍走 print/ stdin 读取）。

## 5. 测试策略

- `render.rs` / `commands.rs`：纯函数单测（`#[cfg(test)]`），喂确定性 XyEvent/输入，断言输出。**优先**，因无终端依赖。
- 端到端：`infra/provider/fake.rs` 的 `FakeModel` 喂确定性事件流，验证 TUI app 状态机正确流转（不渲染真终端，测逻辑层）。
- 不为 MVP 写真终端渲染测试（insta 快照可后续加）。

## 6. 不做的事（防 scope creep）

- ❌ FrameScheduler（帧合并限速）— 等性能问题出现再加
- ❌ EventBroker pause/resume（外部编辑器）— 等编辑器集成再加
- ❌ 多行编辑器（tui-textarea）— MVP 单行输入
- ❌ 表格 holdback、markdown 流式渲染 — 后续 c350/独立变更
- ❌ RemoteDriver / daemon — c325 决策 2 明令 in-process
- ❌ 完整 bootstrap/dispatch 抽取 — c330/c335 职责

## 7. 实施后修复记录（MVP 验收后的迭代）

初版落地后真终端验收暴露 5 个问题，逐个修复并补测试。如实记录，便于回顾。

### F1. EventStream 导致 cursor 查询超时【根因，已修复】
**现象**：`error: The cursor position could not be read within a normal duration`。
**根因**：使用了 crossterm 异步 `EventStream`，它独占 stdin reader，吞掉了 ratatui inline viewport 每次 draw 查询 cursor 的 DSR（`\e[6n`）响应。这是 crossterm 已知问题（[#963](https://github.com/crossterm-rs/crossterm/issues/963)），**非 ratatui inline 的固有缺陷**——ratatui 官方 inline 示例用阻塞 `event::poll`/`read` 完全无此问题。
**修复**：丢弃 `EventStream`，键盘改用独立 `spawn_blocking` 任务跑阻塞 `event::poll`/`event::read`；agent 的异步 `EventStream` 在 spawn 任务里 drain 到 mpsc。主循环的 cursor 查询与键盘读取不抢同一 stdin reader。
**删掉的死路**：曾尝试 EventBroker pause/resume（draw 前 drop stream、draw 后重建），但每次 draw 都 pause/resume 竞态敏感、不可靠，整文件删除。

### F2. cursor 坐标系错误 + 内容未底部对齐【已修复】
**现象**：cursor 错位、`❯` 渲染在 viewport 顶部而非输入行。
**根因**：`set_cursor_position` 接受**绝对终端坐标**（相对终端窗口左上角），但代码传的是相对 area 坐标；且 `Paragraph` 默认从 area 顶部渲染，内容与 cursor 不对齐。
**修复**：cursor 用 `area.y + height - 1`（绝对）+ `UnicodeWidthStr` 算显示宽度；内容底部对齐渲染（`content_area` 偏移到 area 底部 N 行）。
**测试**：`cursor_after_*` + `streaming_shows_thinking_above_input` 用 `TestBackend::assert_cursor_position` 验证。

### F3. 中文输入 cursor 错位【已修复】
**现象**：输入中文时 cursor 位置偏少。
**根因**：cursor 偏移用 `chars().count()`，但中文字占 **2 列显示宽度**。
**修复**：改用 `unicode_width::UnicodeWidthStr::width`（ratatui 内部也用它）。新增 `unicode-width` 依赖。
**测试**：`cursor_after_chinese_input`（`你好` → col 6）。

### F4. CJK 字符间出现空格【已修复】
**现象**：中文之间有多余空格。
**根因**：`commit_to_scrollback` 用 `line.clone().render(area, buf)`，ratatui 对双宽字符的第二格写入 filler cell（存储为空格），提交到 scrollback 时这些 filler 被当作真实空格输出。
**修复**：手动逐格写 buffer——双宽字符第二格 `set_symbol("")`，避免 filler 泄漏到 scrollback。

### F5. spinner 速度不均匀【已修复】
**根因**：spinner 在 `handle_xy_event` 里 tick，速度跟随事件到达率（事件密集时快、稀疏时停）。
**修复**：spinner 改由 `tokio::time::interval(120ms)` 的 `Msg::Tick` 定时驱动，与事件到达率解耦。`handle_xy_event` 不再 tick。

### F6. 输入行颜色 + 流式时 cursor 行为【已修复，需求调整】
**需求变更**：输入框应**永远激活常驻**（流式时也可输入、回车打断当前对话发新 prompt），cursor 始终在输入框、不跟随流式文字；输入用正常色而非蓝色。
**修复**：输入行流式时也用 `❯`（非 `…` 禁用态）；cursor 始终设到输入行；流式时回车 Submit 先 `driver.abort()` + `cancel` 再发新 prompt；输入色改 `Style::default()`（仅背景块，无前景色）。

### 验证基础设施（关键交付）
建立了 **`TestBackend::assert_cursor_position`** 驱动的渲染测试（render.rs `cursor_tests` 模块，8 个测试），覆盖：cursor 坐标（含中文）、流式/空闲布局、底色块、thinking indicator 位置。这是「不再盲改、测试驱动」的基础——对标 codex 的 VT100Backend / pi 的 @xterm/headless。

### 已知遗留（未在本变更处理，记录供后续）
1. **输入框「未置底」视觉错觉**：`Viewport::Inline(3)` 上方空白给人没到底的错觉。后续考虑 background fill / divider。
2. **insert_before 闪烁（果冻效应）**：部分终端上每 TextDelta 的 `\n` 提交触发清+重绘。后续考虑 `scrolling-regions` feature 或批处理。
3. **用户消息不上行**：Enter 提交后用户输入文字未提交到 scrollback。曾尝试加 commit 但回退。
4. **TurnEnd 后回复留尾区**：下次提交才上行，同上回退。

## 8. 依赖架构决策（c355 前瞻）

经调研（ratatui crate-split 架构）：我们用到的 `Terminal`/`Viewport::Inline`/`insert_before`/`Frame::set_cursor_position`/`TestBackend`/`Widget` trait/`Buffer`/`Layout`/`Style`/`Text`/`Line`/`Span` **全部在 `ratatui-core`**。umbrella `ratatui` crate 只是 re-export shim + 几个便利函数（`try_init_with_options`/`restore`/`DefaultTerminal`，~15 行可自写）。多数内置 widget（流式 transcript、带 cursor 输入框、spinner、slash 弹窗）我们用不上、得自写。

**决策**：登记为独立变更 `c355-switch-to-ratatui-core-backend`（依赖 c340 完成），从 umbrella 切到 `ratatui-core` + `ratatui-crossterm`，启用 `scrolling-regions`。这是纯依赖瘦身（无行为变化），不塞进 c340。详见 c355 proposal。
