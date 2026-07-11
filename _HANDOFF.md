# _HANDOFF — 双轨交接（非规范）

> 最后更新：2026-07-11（轨 P：c570 已归档；死代码分诊 + Overlay restore 草案）
> 分支语境：轨 A → `feat/tui-dev`；**轨 P** → worktree 分支 `polish/tui-components`（可开 PR）。
> **本文是临时交接/进度板，不是 SSOT。** 稳定边界：各层 `AGENTS.md`、`docs/architecture/`、`llmanspec/changes/`。

---

## 〇、三轨

| 轨 | 范围 | 状态 | 冲突面 |
|---|---|---|---|
| **P · 包 / demo / DESIGN** | `packages/xylitol-tui`、`agent_demo`、可选 `src/app/tui/DESIGN.md`+`design/*`（只文档） | 提案队列空；待开 PR | 与轨 A 几乎零冲突 |
| **A · 业务核心** | `src/{domain,runtime_protocol,agent,infra,app/core}`、c500–c525 | **可 apply**（任务未勾） | 主 crate；勿与轨 P 同改产品 `src/app/tui` |
| **B · 产品 TUI** | `src/app/tui` 接线、c465–c493 | **冻结** | 开闸 = 轨 A 相关落地 + 用户明确开闸 |

**原则**

1. 轨 P：组件化、可单独验证；缺能力先改包，**禁止**在冻结期堆产品 bridge。
2. 轨 A：按波次 apply；Print 主线可回归。
3. 轨 B：purpose-draft 可改文档；**勿 apply** 直至开闸。

**Worktree**：轨 P → `polish/tui-components`；轨 A → `feat/tui-dev`（或 `refactor/core-export`）。

---

## 一、轨 A — 业务优化（本仓主线）

| 波次 | Change | 说明 |
|---|---|---|
| A0 | **c520** | XyEvent 闭集护栏（轻） |
| A1 | **c500** → **c510** | 精选 pub use + 删死包装 → domain 去 JsonSchema |
| A1′ | **c505** | Provider 单路径（可与 A1 并行） |
| A2 | **c525** | 异步队列 + QueueUpdate 单通道（**轨 B 开闸 P0**） |
| A3 | **c515** | MCP 配置化（可后置） |

产品语义图：`docs/architecture/`。短索引：`_NOTE.md`。

---

## 二、轨 P — 包 / DESIGN / demo（worktree）

### 本批已收口（`polish/tui-components`）

| Change | 主题 | 归档 |
|---|---|---|
| c530 | Markdown token-efficient 渲染 | `archive/2026-07-11-c530-…` |
| c535 | agent_demo Command plate | `archive/2026-07-11-c535-…` |
| c540 | Diff CJK wrap + SBS 空半栏 | `archive/2026-07-11-c540-…` |
| c545 | CompletionSource + `$` 行内 | `archive/2026-07-11-c545-…` |
| c550 | Expandable Head hint / 零宽安全 | `archive/2026-07-11-c550-…` |
| c555 | DESIGN playground token sync + MD slot | `archive/2026-07-11-c555-…` |
| c560 | TreeSelector 空态 + 过滤选中稳定 | `archive/2026-07-11-c560-…` |
| c565 | ChoicePrompt（Ask 单/多选 + Other + Tabs） | `archive/2026-07-11-c565-…` |
| c570 | Dark/Light `Palette` + demo `/theme` + playground scheme | `archive/2026-07-11-c570-…` |

跟进（无独立 change）：Markdown 弱终端色强调；列表折行；窄宽 clamp；Atoms plates；死代码分诊（见下）。

**下一动作**：开 PR 合入 main；合入后再推进 Overlay focus-restore（**c575** purpose-draft）。

### 目标 / 工作方式

1. DESIGN 可 HTML 预览 → 2. 包组件单测/snapshot → 3. demo 接线 → 4. 短 PR。
**禁止**：无预览/无单测就大改多组件；在冻结的 `src/app/tui` 实现产品 bridge。

### 草案 / 候选

| 状态 | 项 | 备注 |
|---|---|---|
| purpose-draft | **c575** Overlay focus-restore（D08） | **合入 main 后再 apply**；见 `llmanspec/changes/c575-…` |
| 可选 | 包内继续小打磨（导出面 / harness） | 先对齐再 propose |

### 已锁定产品决议（demo 应对齐）

| 主题 | 决议 |
|---|---|
| Esc | 流中 = abort（清 steer，留 follow_up） |
| Ctrl+C | 有输入→清编辑器；空→退出 |
| 流中 Enter / Alt+Enter | steer / follow-up |
| Status | idle **0 行** |
| Diff | word-level；宽屏可 L/R；SBS 无行底 |
| Slash MVP | `/exit` + `/model`（产品开闸后） |
| 会话树 | demo 活树；产品 c491 **stub 冻结** |
| Theme | 产品 MVP **固定暗色**；demo `/theme` + 可选 `THEME_AUTO`（COLORFGBG only） |
| 高亮 | demo/产品同一 syntect 回调；包只收回调 |

### `agent_demo` 键位（摘要）

| 键 / 命令 | 作用 |
|---|---|
| 流中 Enter / Alt+Enter | steer / follow-up |
| Esc | abort |
| Ctrl+C | 清输入 / 空则退 |
| 双 Esc | 会话树 |
| `!` / Ctrl+G | bash 边框 / `$EDITOR` |
| `/theme [dark\|light\|toggle]` | 显式换肤（关 auto） |
| `XYLITOL_AGENT_DEMO_THEME_AUTO=1` | COLORFGBG 探测（勿写 OSC 进 crossterm 环） |

### 死代码分诊（2026-07-11 · 包 `xylitol-tui`）

| 符号 | 类 | 处置 |
|---|---|---|
| `ToolBlockStatus::ansi_bg_param` | 真死 | **删** |
| `AtPathSource.fd_path` / `new_with_fd` 未读字段 | 真死 | **删字段**；`new_with_fd` 退化为 `new`（fd 仍在 Combined 路径） |
| `UndoStack::clear` 上的 `allow` | 误标 | **去 allow**（editor submit 已用） |
| harness `allow(dead_code)` | 预留 | 保留（跨 test target API） |
| Kitty 常量 / `enable_modify_other_keys` | 预留 | 保留（pi 对等 / route-B） |
| `AnsiCodeTracker::{clear,has_active_codes}` | 预留 | 保留（extract_segments 对等） |
| example `main` allow | 预留 | 保留（lib+example 编译形态） |

---

## 三、轨 B — 产品 TUI（冻结）

```text
已归档：c460 host · c461 队列 seam · c491 stub-only
开闸后：c465 bridge → c475 chrome / c480 input → c485 垂直切片
paused：c470 Codex TranscriptView
后置：c490 trust · c492 bash · c493 compaction/retry
```

开闸前 P0：`c465` design + **c525**。冻结期内允许：harness 回归、DESIGN、包内通用缺口。

---

## 四、SSOT 指针

| 主题 | 路径 |
|---|---|
| 分层 / 导出 / 冻结 | 根 + `src/AGENTS.md`、`src/app/tui/AGENTS.md` |
| 产品架构图 | `docs/architecture/` |
| 包边界 / vs pi | `packages/xylitol-tui/AGENTS.md`、`PI_DELTAS.md` |
| 视觉 | `src/app/tui/DESIGN.md` + `design/` |
| How-to | `write-tui`、`test-tui-harness`、`write-surface`、`audit-dead-code` |
| 轨 A 索引 | `_NOTE.md` |
