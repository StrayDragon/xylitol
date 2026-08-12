# Tasks: c2080-add-append-only-subagent-tui

> **前置**：[`c2071`](../archive/2026-08-12-c2071-update-app-tui-host-mode-b-only/) 已归档。
> **本阶段**：Designed / pre-start（**未** Branch binding）。下方 Specs / Apply 在 `change start` 后执行。
> **决策**：见 `design.md` + 下文 Open Questions（已钉）。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 0 规划壳 Designed | ✅ | proposal + design + tasks；OQ 全钉 |
| 1 Branch binding | ⬜ start 后 | `sdd/c2080-…` |
| 2 Specs landing | ⬜ start 后 | `app-tui-append-only` + ath30 主语澄清 |
| 3–5 实现 / 探针 | ⬜ apply | 面 + spill + harness |
| 6 validate / verify | ⬜ | apply 门禁 |

---

## Start readiness

| 检查 | 结果 |
|---|---|
| c2071 归档 | ✅ |
| Open Questions 全钉 | ✅ |
| design 含入口 / InteractionMode / spill / 键位 / 复用表 | ✅ |
| 未改 live specs / 未写应用代码 | ✅（硬约束） |
| **`ready_for_start`** | **`true`** |

---

## Open Questions（已钉）

| # | 问题 | 钉 |
|---|---|---|
| 1 | 载体 | **库 `InteractionMode::Inline`**；产品名 append-only surface，不教「Inline 模式」 |
| 2 | 嵌套形态 | **本票 = 独立 `SurfaceMode::AppendOnly` 入口**；AO 内嵌套面板后置 |
| 3 | ath30 | **并列新 capability**；ath30 仅澄清「主会话产品 TUI」，不削弱 AO-only |
| 4 | 键位闭集 | abort（Busy Esc）+ 退出（`/exit` 与/或空闲双 Ctrl+C）+ 原生滚动；**无** spill 专用键、**无** Ctrl+O/fold/slash 发现 |
| 5 | Sub-Agent roadmap | **只交面 + 架构探针**；编排 M1 另票；harness 用 ScriptedDriver |

块帽（随 OQ 一并钉）：tool/bash/assistant **3** 行；write/diff 类 **5** 行；均不可展开。

---

## 0. Designed — ✅

- [x] 0.1 读 proposal + `research/pi-cursor-subagent-ui.md` + write-surface / XyDriver / `full_output_path`
- [x] 0.2 写 `design.md`（入口、InteractionMode、spill、键位、复用表）
- [x] 0.3 钉 Open Questions；`ready_for_start=true`

---

## 1. Branch binding — start 后

- [ ] 1.1 干净默认分支：`llman sdd change start c2080-add-append-only-subagent-tui`
- [ ] 1.2 确认 `llman sdd show … --json`：attached / stage full（尚未 specsLanded）

## 2. Specs landing — start 后

- [ ] 2.1 新建 live `app-tui-append-only`：入口、Inline 载体、无折叠、块帽 3/5、spill 目录契约、键位闭集、架构探针（复用 XyDriver）
- [ ] 2.2 `app-tui-host` ath30：主语澄清为**主会话**产品 TUI（MUST NOT 把 AppendOnly 读成 ath30 例外削弱）
- [ ] 2.3 场景：优先 `feature: false` unit + harness 覆盖；避免无意义 dual-write
- [ ] 2.4 commit Specs landing → `readyToImplement=true`

## Apply backlog（`llman-sdd-apply`；start 前勿改代码）

### 3. 面入口 + 瘦 host

- [ ] 3.0 落点区域 `audit-dead-code`（write-surface 步骤 1）
- [ ] 3.1 `SurfaceMode::AppendOnly` + CLI 显式进入（不抢 TTY 默认 AO）
- [ ] 3.2 瘦 host：`XyDriver::run` → bridge → append-only layout；库 Inline 生命周期
- [ ] 3.3 键位闭集接线；禁止注册主线 fold/slash/plate

### 4. Spill + 矮帽

- [ ] 4.1 session 旁唯一 spill 目录；`full_output_path` 指向其内文件
- [ ] 4.2 UI 行帽 3/5；溢出写 spill + `[Full output: …]`；无展开交互
- [ ] 4.3 与工具硬截断路径对齐（最小 infra 改动若必需）

### 5. 架构探针 + 验证

- [ ] 5.1 ScriptedDriver harness：流式/工具/spill/abort/exit
- [ ] 5.2 审查复用/不复用表（import 边界）
- [ ] 5.3 `llman sdd validate --strict`；相关测绿 → verify

## 实现顺序

```text
0 Designed ✅
  → 1 change start
  → 2 Specs landing → readyToImplement
  → 3 面入口 + Inline host
  → 4 spill + 矮帽
  → 5 harness + 架构审查 → verify → archive
```
