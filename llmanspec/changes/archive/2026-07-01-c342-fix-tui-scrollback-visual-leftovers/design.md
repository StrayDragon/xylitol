# c342 Design — 修复 c340 TUI 的 3 项遗留（#1/#3/#4）

本变更只修 c340 §7 记录的 3 项视觉/行为遗留（第 2 项闪烁归 c341）。设计目标：让 inline REPL 对话历史完整、视觉干净。

## 1. 修复 #3：用户消息上行

### 问题定位
`mod.rs:154-177` 的 `Submit` 分支：
```rust
InputOutcome::Submit(prompt) => {
    // ... abort 处理 ...
    let stream = driver.run(&prompt).await;  // ← prompt 取走，但从未 commit 到 scrollback
    app.start_stream();
    ...
}
```
`take_input()`（`app.rs:45`）已把输入移出，但没渲染。c340 design §7 说「曾尝试加 commit 但回退」——推测回退原因可能是 commit 后 tail 又 draw 导致重复显示，或样式没选好。

### 修复方案
在 `driver.run` **之前** commit 用户消息（这样用户消息在回复流之前，符合对话顺序）：
```rust
InputOutcome::Submit(prompt) => {
    if app.is_streaming() { driver.abort(); cancel.cancel(); app.end_stream(); }
    // 用户消息上行（修复 #3）
    let user_line = Line::styled(format!("❯ {prompt}"), theme::palette().user_prompt());
    term.commit_to_scrollback(&[user_line])?;
    let stream = driver.run(&prompt).await;
    app.start_stream();
    ...
}
```
- `theme.rs` 加 `user_prompt()` token：比 assistant 更显眼（如 bold 或 primary 色），区别于流式回复。
- 注意：流式中 Enter 打断重发的场景（c340 F6），用户消息也要上行（用户看到自己打断了）——放在 abort 之后、run 之前，顺序自然。

### 测试
- 纯逻辑层：`TuiApp` 暴露 `take_input()` 返回的 prompt，测试「提交后 scrollback 含用户行」。可在 app.rs 加 `last_user_prompt()` 或在 mod 层验证（mod 层难单测，因依赖 term；改用 render 层断言 commit_lines_for 能产出用户行，或在 app 层加一个记录）。
- 实际：用户消息 commit 在 mod.rs（需 term），难纯单测。用手动验收 + 在 app.rs 加一个 `commit_user_line(prompt) -> Line` 纯函数单测。

## 2. 修复 #4：TurnEnd 同帧清尾 + 上行

### 问题定位
`mod.rs:209-221`：
```rust
Msg::Xy(ev) => {
    let is_end = app.turn_done(&ev);
    let lines = app.handle_xy_event(*ev);  // TurnEnd 时 drain pending → out
    if !lines.is_empty() { term.commit_to_scrollback(&lines)?; }
    term.draw_tail(app)?;                   // pending 已空 → tail 清空
    if is_end { app.end_stream(); term.draw_tail(app)?; }
}
```
时序分析：`handle_xy_event(TurnEnd)` 把 `pending` drain 到返回值（`app.rs:127-130`），然后 commit + draw_tail。此时 `current_streaming_line()`（读 `pending`）已 None → tail 不显示流式文本。**理论上同帧完成**。

c340 §7 说「下次提交才上行」——推测真实根因可能是：TurnEnd 事件到达时，`pending` 里**还有未达 `\n` 的最后一行**（`flush_complete_pending` 只在 `\n` 边界 commit，TurnEnd 才 flush 剩余，`app.rs:124-133`）。这个 flush 的行**确实**在 TurnEnd 这帧 commit 了。但如果 `end_stream()` 在 commit 之前被某路径调用，或 draw_tail 在 commit 之前跑，会撕裂。

**需实施时验证**：加日志或断言确认 TurnEnd 帧 commit 的行数 == pending 末尾剩余。若时序正确则「留尾区」可能是视觉错觉（tail 在下一帧才被新输入覆盖），修复方向是 TurnEnd 后立即 `draw_tail` 清空（已做）。

### 修复方案
- 确认 `handle_xy_event(TurnEnd)` 的返回行在 `end_stream()` **之前** commit（当前顺序对：先 handle→commit→draw→end_stream→draw）。
- 关键修复：TurnEnd commit 后、`end_stream` 后的第二次 `draw_tail`（`mod.rs:219`）确保 tail 完全清空（无 streaming text、无 indicator）。当前 `end_stream` 设 `streaming=false`（`app.rs:140-143`），`draw_tail_frame` 在 `!is_streaming` 时不画 indicator/streaming line（`render.rs:36-49`）——**应已清空**。
- 若仍有残留，根因在 `pending` 未在 TurnEnd 彻底清——检查 `app.rs:124-133` 的 drain 是否覆盖所有 pending（含无 `\n` 结尾的）。

### 测试
- `app.rs` 已有 `turn_end_flushes_remaining_pending`（`app.rs:216-219`）测 TurnEnd flush。加强：assert flush 后 `current_streaming_line() == None`。

## 3. 修复 #1：输入框视觉置底

### 问题定位
`terminal.rs:21` `TAIL_HEIGHT=3`，`render.rs:60-66` 底部对齐渲染。空闲时只 1 行（输入行），上方 2 行是 tail 区的空白（viewport 保留但无内容）。

### 修复方案（选项 A：背景填充，倾向）
空闲行用 `input_bg` 填充，让 tail 区视觉上是 3 行连续色块：
```rust
// render.rs draw_tail_frame，渲染 lines 之前先填满 tail 区背景
let bg = p.input_bg();
for y in area.y..area.bottom() {
    for x in area.x..area.right() {
        frame.buffer_mut()[(x, y)].set_bg(bg);
    }
}
// 然后底部对齐渲染 lines（覆盖最底 N 行）
```
或更精细：只填输入行及上方紧邻的空白行，indicator 行用不同背景。

**与 c341 的交互**：c341 把 Paragraph 换成 Line::render，本修复的背景填充用 `frame.buffer_mut()[(x,y)].set_bg()`（直接 Buffer 操作），与 c341 方向一致（自建组件直接写 Buffer）。**建议 c342 在 c341 之后实施**（这样 buffer_mut 操作已是既定模式），或独立实施（buffer_mut 本就是 ratatui_core 公开 API）。

### 测试
- `render.rs` 加测试：空闲态断言 tail 区所有 cell 的 bg == input_bg（或至少输入行上方紧邻行）。
- 复用 `TestBackend` + `buffer()` 遍历 cell。

## 4. 风险与缓解

### R1：用户消息 commit 时序与流式首行竞争【低】
`commit` 后立即 `driver.run` + `start_stream`，首个 TextDelta 可能在 commit 完成前到。
**缓解**：commit 是同步 `insert_before`，返回后才 run；mpsc 异步，无竞争。

### R2：背景填充影响 cursor 测试【低】
填充 bg 改变 cell 状态，可能影响 `input_line_has_background` 之外的断言。
**缓解**：c340 测试只断言特定 cell 的 bg/cursor，填充不改变这些。新加测试时注意。

### R3：修复 #4 根因未明【中】
「下次提交才上行」可能是事件时序而非渲染时序。
**缓解**：实施时先加诊断（临时日志或测试断言 TurnEnd 帧 commit 行数），定位再修，不盲改。

## 5. 实施顺序建议

1. 先修 #3（用户消息上行）——最影响可用性，方案明确。
2. 再修 #4（TurnEnd 清尾）——需先定位根因。
3. 最后修 #1（视觉置底）——纯视觉，风险最低。
每个修复后跑 c340 回归测试 + 加新测试。

## 6. 不做的事（防 scope creep）

- ❌ 闪烁（#2）—— c341。
- ❌ 组件目录重组 —— c350。
- ❌ 多行输入 —— c340 已 defer。
- ❌ 改 driver/agent —— 只动 TUI 层。
