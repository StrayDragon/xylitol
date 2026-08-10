# 人类验证清单（c2020 · S6 配套）

自动化：`#[ignore]` PTY 经 `just test-tui-e2e-pty`；默认 `just qa` 不强制。

## Kitty（或等价现代终端）

1. 构建并进入产品 TUI 或 `just demo-tui`。
2. **未**设 `XYLITOL_TUI_MOUSE` 时：终端拖选复制仍可用（默认关 capture）。
3. `XYLITOL_TUI_MOUSE=1 just demo-tui`（或同 env 跑产品）后：晃鼠标 **无明显空转闪烁**（Moved 未刷帧）。
   - **无法普通拖选 / 滚历史是预期**（应用抢了 mouse reporting）。foot / Kitty / Ghostty 同理。
   - foot：按住 **Shift** 再拖可选中（`selection-override-modifiers`，默认 Shift）；滚轮事件进应用——本波产品未处理 wheel，滚屏不会动。
4. `/exit`（产品）或清空编辑器后 `Ctrl+C`（`agent_demo`）正常退出后：shell 下拖选恢复；无残留 mouse reporting 怪异行为。

## tmux（可选）

- 同上；若 tmux 吞鼠标，记录为环境限制而非产品 bug。

## 不在本 change 验收

- 点击折叠三角、leader 数字（c2030/c2040）。
