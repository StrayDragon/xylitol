---
depends_on:
- c2070-add-package-tui-dual-interaction-modes
blocks: []
branch: sdd/c2071-update-app-tui-host-mode-b-only
base_sha: 1dd5de3a53e28365099c2351abd0a767bbfab688
checkpointed: true
checkpoint_sha: 1dd5de3a53e28365099c2351abd0a767bbfab688
---

# 产品 TUI 固定 ApplicationOwned（ath30 AO-only）

> **拆自** [`c2070`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/proposal.md) 任务 7.5。
> **状态**：Full + Specs landed（ath30 AO 默认）；实现见 `tasks.md` 块 2–4。

> **一句话**：产品 `src/app/tui` **MUST** 仅 ApplicationOwned；改写 `ath30` 默认；删产品「默认 Inline」叙事；库仍暴露 Inline 为遗留入口。**无**运行中热切。

## Why

战略已拍：产品不维护双模式 UX；退出 AO 后靠 dump 保留主屏可翻内容，不是切回 Inline。
c2070 已归档库双入口；本票把 live `ath30` **默认翻成 ApplicationOwned** 并跟产品 host。

## What Changes

1. **Specs landing**：`ath30` → 产品 MUST 固定/默认 ApplicationOwned；废「缺省 Inline」。
2. **产品 host**：`TuiRunOptions` 默认 `ApplicationOwned`；产品测试 / Fake / harness 跟默认。
3. **文档**：`src/app/tui/AGENTS.md`、相关 PI_DELTAS（如 D16）与产品人验清单对齐 AO-only。
4. **非目标**：库删 Inline；恢复 mid-session 换模式 API；fold 点击（仍 fold 族）。

## Capabilities

- `app-tui-host`（ath30 默认翻转）
- 按需触及产品 TUI 相关 feature / harness（不扩库双模式合约）

## Impact

| 层 | 影响 |
|---|---|
| live specs | `ath30` 缺省从 Inline → ApplicationOwned |
| `src/app/tui` | `TuiRunOptions` 默认 + 测试期望 |
| 库 `xylitol-tui` | **无**（构造入口已在 c2070） |
| 下游 | 产品可点折叠的正式 UX 依赖本 change 默认已开 ApplicationOwned |

## 依赖

```text
c2070 ✓ archived（库 ApplicationOwned + 启动绑定 + 禁止热切）
  └─ c2071（本 change：产品 ath30 默认 AO）
```

### 库侧已就绪（c2070）

| 需求 | API / 事实 |
|---|---|
| 启动绑定 AO | `TUI::with_interaction_mode` / `ApplicationOwnedTui` / `HostSession::new_product_ui_with_meta_mode` |
| 禁热切 | 无 `apply_interaction_mode` |
| 退出留主屏可翻 | `finish` / `finish_application_owned` + dump |
| Host 清单 | `packages/xylitol-tui/AGENTS.md` § ApplicationOwned host checklist（ptim14） |

## Out of scope

- 库双入口结构重构（属 c2070）
- Mid-session Inline↔AO 热切（**明确不做**）
- 产品 lab/env 逃生回 Inline；dump 用户 opt-out
- Activity fold / 点三角（`c1760` / `c2040`）

## Open Questions（已钉）

1. lab/env 逃生回 Inline → **不做**；库 Inline 仅 lab/demo。
2. dump opt-out → **不**暴露用户开关；沿用库默认 dump-on。
3. 终端签字 → 任务 3.2；不挡 Specs / apply 起步。

## Further Notes

- 调研：[`keep-or-drop-inline-mode.md`](../archive/2026-08-12-c2070-add-package-tui-dual-interaction-modes/research/keep-or-drop-inline-mode.md)。
- 勿再引入 `apply_interaction_mode`。
