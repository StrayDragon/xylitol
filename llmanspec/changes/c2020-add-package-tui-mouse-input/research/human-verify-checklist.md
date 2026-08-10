# 人类验证清单（c2020 · S6 配套）

自动化：`#[ignore]` PTY 经 `just test-tui-e2e-pty`；默认 `just qa` 不强制。

## Kitty（或等价现代终端）

1. 构建并进入产品 TUI 或 `just demo-tui`。
2. **未**显式 Enable 时：终端拖选复制仍可用（默认关 capture）。
3. 显式调用 enable（apply 后的开关 / 调试 API）后：晃鼠标 **无明显空转闪烁**（Moved 未刷帧）。
4. `/exit` 或正常退出后：shell 下拖选恢复；无残留 mouse reporting 怪异行为。

## tmux（可选）

- 同上；若 tmux 吞鼠标，记录为环境限制而非产品 bug。

## 不在本 change 验收

- 点击折叠三角、leader 数字（c2030/c2040）。
