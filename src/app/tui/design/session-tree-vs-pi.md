# Design notes — Session Tree vs pi

> 调研源：`../pi/packages/coding-agent/.../tree-selector.ts` + `interactive-mode.ts`。
> **刻意不做 / 不得回退**：见 [`../PI_DELTAS.md`](../PI_DELTAS.md)（如 travel 分支摘要 A01）。本文件是对照备忘，不是进度板。

## 槽模型

pi / xylitol：树 **替换 editor 槽**（`showSelector`），非居中 overlay。

## 操作对照

| 能力 | pi | xylitol |
|---|---|---|
| ↑↓ / Enter / Esc | ✓ | ✓ |
| ←→ / PgUp/PgDn、搜索、filter、fold、label | ✓ | ✓（产品已接线） |
| Enter travel → editor 预填 | ✓ | ✓ |
| 树内 / 会话 fork | ✓ `/fork` user 选择器 | ✓ `/session-fork` 新 session（A02） |
| 流中 steer / follow-up | ✓ | ✓ |

包吃 `TreeNode { id, label, children, annotation?, kind? }`；**kind 前缀由主题画**，host **不**把 role 字符串烘焙进 `label`。
