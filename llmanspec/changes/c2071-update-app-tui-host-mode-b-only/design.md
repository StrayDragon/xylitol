# Design: c2071-update-app-tui-host-mode-b-only

## 目标边界

| 在范围 | 不在范围 |
|---|---|
| live `ath30` → 产品 MUST 固定 Mode B | 库双入口结构（属 [`c2070`](../c2070-add-package-tui-dual-interaction-modes/)） |
| 产品 `TuiRunOptions` 默认 ApplicationOwned | 删库 Inline / 差分引擎 |
| 产品 harness / Fake / 文档对齐 B-only | fold 点击（`c2040`）、activity fold（`c1760`） |
| 按 c2070 ptim14 清单接线 host | 在 c2070 分支上改 ath30 |

## 为何从 c2070 拆出

1. **带宽**：c2070 需先把单 `TUI` if 分支收敛成可复用 ApplicationOwned 入口；产品切默认若并行，易抄 demo 胶再返工。
2. **合约闸**：ath30 是产品行为 Specs landing，应单独 Branch binding，不与库结构债同 commit 史纠缠。
3. **风险**：B-only 去掉会话中原生 scrollback 逃生阀；人验 / SSH 签字应挂在产品票，不挡库 archive。

## 依赖与顺序

```text
c2070 archive（库入口 + ptim14 可抄）
  → c2071 start → ath30 Specs landing → host 默认 B → 文档/人验 → archive
```

**硬闸**：`depends_on: c2070`；未归档前不可 apply。

## 产品接线原则

- 只走库文档化的 ApplicationOwned host 环（`begin` / `dispatch` / `idle_tick` / `finish`）。
- **禁止**把 `agent_demo_impl` 私有坐标变换当 SSOT。
- Inline 保留为库 lab；产品不暴露用户模式开关（内部 env 逃生若保留须显式钉「非用户设置」）。

## 验证

| 层 | 内容 |
|---|---|
| Specs | ath30 + 相关 feature 绿 |
| 产品单测 / Fake | 默认即为 Mode B 生命周期 |
| 人验 | Ghostty / tmux / SSH 签字板（任务 3.2） |

## 开放项（propose 可钉）

见 proposal Open Questions：lab 逃生阀、dump opt-out、终端签字人。
