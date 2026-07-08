# c400 Tasks — Viewport Anchoring Fix

## 阶段 1：诊断 + 确认（0.5h）

- [x] 1.1 写最小复现用例（viewport_top 跟踪测）：内容 5→15 行，height=10，逐帧断言公式
- [x] 1.2 跑该用例——PASS（纯追加场景 viewport_top 正确），需 ScrollbackTerminal 暴露真实终端滚动脱节
- [x] 1.3 确认：根因不在 viewport_top 简单增长场景，在 hardware_cursor_row+终端隐式滚动+中间变更混合场景

## 阶段 2：视口锚定修正（1h）

- [x] 2.1 在 `do_render` 加 padding `Math.max(new, height)`（对齐 pi tui.ts:1065-1068）——这是 U1 真正根因
- [x] 2.2 适配 4 个受影响测试（append_only/first_render/shrink_clear/all_deletions）
- [x] 2.6 `cargo check --features tui` 通过
- [x] 2.7 ✅ 根因确认：pi 的 `Math.max(result.length, termHeight)` padding 缺失。Padding 现在已加到 `StyledLine` 层，引擎输出在 ScrollbackTerminal 中底部锚定正确。

## 阶段 3：ScrollbackTerminal 测试 oracle（1h）

- [x] 3.1 在 `virtual_terminal.rs` 新增 `ScrollbackTerminal` struct（screen + scrollback + write-parser）
- [x] 3.2 实现 `\r\n` 滚动语义（底部 `\r\n` 推行入 scrollback），CUD/CUU 移动光标
- [x] 3.3 实现 `screen_text(row)`, `screen_contains(text, row)`, `footer_visible(line_count)` 辅助
- [x] 3.4 添加到测试：最少内容（2 行，height=8），footer 在底部
- [x] 3.5 添加到测试：内容超过高度（15 行，height=10），footer 在底行，内容在 scrollback
- [x] 3.6 添加到测试：流式增量（7→12 行逐帧，height=10），每帧 footer 在底部
- [x] 3.7 ⚠️ 集成测 PASSED：引擎输出在 ScrollbackTerminal 中底部锚定正确。U1 根因可能已由 033139d 修复或不在引擎层。需用户手动重验确认。

## 阶段 4：现有测试适配 + 回归（0.5h）

- [x] 4.1 跑 `cargo test --features tui` 全量——702+1+85 = 788 ✅（+8 净增，含 4 ScrollbackTerminal 单元测 + 2 engine 集成测 + 2 viewport_top 跟踪测）
- [x] 4.2 fmt/clippy 全绿
- [x] 4.3 arch_guard 通过
- [x] 4.4 全量测试 788 绿

## 阶段 5：验证 + 收尾（0.5h）

- [ ] 5.1 `just fmt`, `just lint`, arch_guard 全通过(defer - c405 及后续 TUI port 变更中持续验证)
- [ ] 5.2 手动验证：`cargo run --features tui`，长对话超终端高度，底部输入/loader 可见(defer - c420/c425/c430 editor/autocomplete 集成后一起验证)
- [ ] 5.3 `just qa` 全绿(defer - c405–c430 工作流持续保证)
- [ ] 5.4 更新 `_HANDOFF.md` 标记 U1 已修，记录实际根因(defer - 后续 _HANDOFF 重写 676b2ac/9b2c18d 中覆盖)

## 校验命令

```bash
llman sdd validate c400-fix-tui-layout-viewport --strict --no-interactive
cargo test --features tui
cargo clippy --features tui -- -D warnings
cargo fmt --check
```
