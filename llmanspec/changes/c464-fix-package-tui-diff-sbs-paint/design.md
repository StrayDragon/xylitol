# Design — c464-fix-package-tui-diff-sbs-paint

## Decision: A

| 路径 | 行底 `diff-*-bg` | 极性 | 词级 |
|---|---|---|---|
| **Unified** | 保留（pad 后套） | fg + `±N` | 现有 `word_change_*` |
| **SBS** | **不套** | fg + 半栏 gutter | 若日后启用：inverse / 仅 span，不叠行底 |

语义分层：

- `tool-*-bg` → 执行 pending/success/error（Diff **header only**）
- `diff-*-bg` → **仅 unified** 行极性辅助
- SBS → 靠左右栏 + fg，避免双绿/双红墙

## Rejected

- B：SBS 极淡行底（仍易与 tool tint 糊）
- C：砍掉 SBS 默认（改动面过大）
