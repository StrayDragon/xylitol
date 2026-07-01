# c342-fix-tui-scrollback-visual-leftovers — Tasks

> 修复 c340 §7 遗留 #1/#3/#4（#2 闪烁归 c341）。每个修复独立可验收。chunk ≤ 1h。

## 0. 前置

- [x] 确认 c340 既有测试全过（回归基线）：`cargo test --features tui --lib tui::`
- [x] 查清 c340 design §7 关于 #3「曾尝试加 commit 但回退」的具体原因（git log/diff 或代码注释），避免重蹈

## 1. 修复 #3：用户消息上行

- [x] `theme.rs`：加 `user_prompt()` token（区别于 assistant；如 primary 色或 bold）
- [x] `app.rs`：加纯函数 `commit_user_line(prompt: &str) -> Line`（可单测）
- [x] `mod.rs` Submit 分支：在 `driver.run` 之前 `term.commit_to_scrollback(&[app::commit_user_line(&prompt)])`
- [x] 流式中 Enter 打断重发路径（`mod.rs:157-161`）：确认用户消息也在该路径上行（abort 后、run 前）
- [x] 单测：`commit_user_line` 产出含 prompt 文本的 Line
- [ ] 手动验收：输入 prompt → scrollback 出现 `❯ <prompt>`(defer → c355-ensure-terminal-restore-on-panic)

## 2. 修复 #4：TurnEnd 同帧清尾

- [x] 定位根因：在 TurnEnd 分支加临时诊断，确认 commit 行数 == pending 末尾剩余；确认 `end_stream` 后 `current_streaming_line() == None`
- [x] 若时序正确（commit + draw 同帧）→ 检查是否 `end_stream` 后未额外 draw（当前有 `mod.rs:219` 第二次 draw_tail，应足够）
- [x] 若根因是 pending 未彻底清 → 修 `app.rs:124-133` 的 TurnEnd drain
- [x] 加强测试：`turn_end_flushes_remaining_pending` 之后 assert `app.current_streaming_line().is_none()`
- [ ] 手动验收：一轮对话结束 → tail 无残留回复文本；回复末行在 scrollback(defer → c355-ensure-terminal-restore-on-panic)

## 3. 修复 #1：输入框视觉置底

- [x] `render.rs draw_tail_frame`：渲染 lines 前，tail 区用 `input_bg` 填充空闲行（直接 `frame.buffer_mut()[(x,y)].set_bg(bg)`）
- [x] 评估：只填输入行+紧邻行，还是全 tail 区（倾向全 tail 区连续色块）
- [x] 加测试：空闲态 tail 区 cell bg == input_bg（遍历 buffer 断言）
- [ ] 手动验收：空闲态输入框视觉贴底，无悬空感(defer → c355-ensure-terminal-restore-on-panic)

## 4. 回归与校验

- [x] `cargo test --features tui --lib tui::` 全过（c340 38 个 + 本变更新增）
- [x] 特别确认 8 个 cursor_tests 无回归
- [x] `just qa` 绿
- [x] `llman sdd validate c342-fix-tui-scrollback-visual-leftovers --strict --no-interactive` 通过
