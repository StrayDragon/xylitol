# Design — c705-add-app-tui-session-tree-e2e

## 决策

| 点 | 选择 |
|---|---|
| 驱动 | 沿用 `spawn_product_fake_ready` + portable-pty |
| 最小绿 | 提交 hi → Fake 回复 → 双 Esc → 见 `Type to search` 与 `fold/unfold` 或 `filters` → Shift+L → 输入 → Enter 见 `[…]` → `/exit` |
| 忽略 | `#[ignore]` + `just test-tui-e2e-pty`（同既有产品 Fake） |

## 验证

`just test-tui-e2e-pty` 含新 case 绿。
