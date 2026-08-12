# Design: c2071-update-app-tui-host-mode-b-only

## 目标边界

| 在范围 | 不在范围 |
|---|---|
| live `ath30` → 产品 MUST **默认** ApplicationOwned | 库双入口结构（属 [`c2070`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/)） |
| 产品 `TuiRunOptions` 默认 ApplicationOwned | 删库 Inline / 差分引擎 |
| 产品 harness / Fake / 文档对齐 AO-only | fold 点击、activity fold；**mid-session 热切** |
| 按 c2070 ptim14 清单接线 host | 恢复 `apply_interaction_mode` |

## 为何从 c2070 拆出

1. **带宽**：c2070 先收口库 ApplicationOwned + 启动绑定 / 禁热切。
2. **合约闸**：默认翻成 AO 是产品 Specs landing，单独 Branch binding。
3. **风险**：AO-only 去掉会话中原生 scrollback 逃生；退出体验靠 dump，人验挂产品票。

## 依赖与顺序

```text
c2070 archive ✅
  → c2071 start → ath30 默认 AO（Specs）→ host 默认 → 文档/人验 → archive
```

**硬闸**：`depends_on: c2070`（已归档 INFO）。

## 产品接线原则

- 只走库文档化的 ApplicationOwned host 环（`begin` / `dispatch` / `idle_tick` / `finish`）。
- 模式在 `new_product_ui_with_meta_mode` / `TuiRunOptions` **启动时**绑定；**禁止** mid-loop 换模式。
- **禁止**把 `agent_demo_impl` 私有坐标变换当 SSOT。
- Inline 保留为库 lab；产品不暴露用户模式开关；不暴露 dump opt-out。

## 验证

| 层 | 内容 |
|---|---|
| Specs | ath30 默认 AO；unit scenario；`validate --strict` |
| 产品单测 / Fake | 默认即为 ApplicationOwned 生命周期 |
| 人验 | Ghostty / tmux / SSH 签字板（任务 3.2） |

## Open Questions（已钉）

见 `tasks.md`「Open Questions（已钉）」：无产品 Inline 逃生阀、无 dump 用户开关、终端签字不挡 Specs。
