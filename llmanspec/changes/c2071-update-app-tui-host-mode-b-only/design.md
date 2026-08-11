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

## 行为基线（切换默认 AO：尽量一致）

> Apply 硬约束：除下列 **必然变化 / 新增机制** 外，产品可观察行为 MUST 与今日 Inline 默认路径保持一致。

### 应保持一致（不因默认 AO 改语义）

| 面 | 说明 |
|---|---|
| Driver / ReAct / slash / bang / ask | 编排与 `XyEvent` 扇入不变 |
| 键位与 InputListener | Ctrl+C / Esc / idle Enter 等产品监听不变 |
| UiRoot chrome | status / editor / footer / queue / toast 布局语义不变 |
| harness H1–H9 业务断言 | 提交→流式/工具→steer→abort→/exit 等 **业务结果** 不变 |
| `finish()` 调用点 | 仍走 `session.tui.finish()`；库按模式分发（AO→`finish_application_owned`） |
| TerminalGuard | 产品仍 **不**读 `XYLITOL_TUI_MOUSE` |

### 必然变化（AO 机制，非回归）

| 面 | Inline（旧默认） | ApplicationOwned（新默认） |
|---|---|---|
| 缓冲 | 主屏差分 + 终端 scrollback | alt-buffer + 应用视口 |
| 选区 | 终端原生 | 应用选区（transcript dock 排除 + Editor 独立） |
| Mouse | 默认不 capture | 会话 begin 后 capture；Moved 纪律不变 |
| 退出 | `finish_inline` 停 TTY | AO teardown + **dump 会话到主屏 scrollback**（可读历史） |
| dock | 无 | 每帧 `sync_dock_rows`；Editor `set_screen_origin` |

### 额外验证（apply 新增/改写测试）

| 测 | 意图 |
|---|---|
| 改写 `interaction_mode_defaults_to_inline` → 默认 AO + session active | ath30 |
| 既有 AO dock / ath31 Copied 测 | 回归不丢 |
| harness `h8_exit_*` | 断言 `finish`/stop；**不**钉死 `finish_inline` 符号 |
| 可选：dump 后主屏可读（库已有测；产品冒烟 SHOULD） | ath30 退出可读 |

### 明确不改（本 change）

fold 点击、activity fold、库删 Inline、热切 API、产品 Inline lab 开关。

## 验证

| 层 | 内容 |
|---|---|
| Specs | ath30 默认 AO；unit scenario；`validate --strict` |
| 产品单测 / Fake | 默认即为 ApplicationOwned 生命周期；**业务 harness 绿** |
| 人验 | Ghostty / tmux / SSH 签字板（任务 3.2） |

## Open Questions（已钉）

见 `tasks.md`「Open Questions（已钉）」：无产品 Inline 逃生阀、无 dump 用户开关、终端签字不挡 Specs。
