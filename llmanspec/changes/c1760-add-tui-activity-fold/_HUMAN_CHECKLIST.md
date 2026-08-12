# Human checklist — c1760 activity-fold（可选）

最短人验（TTY 产品 TUI）：

1. **冷 rebuild**：打开含 ≥3 轮中间工具墙的会话 → 最旧段应见 L2 摘要行（`▸`/`>` + 计数 + `(Alt+Shift+E)`），User / 最终 Assistant 仍在。
2. **展开栈**：`Alt+Shift+E` → 最近 L2/L3 升一级；再按可回到细账（L0）；此时 `Alt+E` / 三角应恢复影响该段。
3. **收纳栈**：对已进过折叠的段 `Ctrl+Alt+Shift+E` → 降一级；近窗从未折叠的细账按键应**静默**。
4. **分层**：段在 L2 时按 `Alt+E` → 该段摘要行外观不变。
5. **无段鼠标**：点击 L2/L3 摘要行**不应**切换段 level（仅 L1 三角可点）。
6. **弱终端**：三修饰收纳若无效 → 改 `keybindings.json`（无第二官方默认）。

自动化已覆盖 att23–att28 harness；本清单仅补真实终端手感。
