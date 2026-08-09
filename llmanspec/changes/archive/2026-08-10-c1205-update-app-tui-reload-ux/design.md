# Design: c1205 /reload 进行中 UX

## Problem

今日 `effects/slash::handle_reload` 在单一 `drain_pending` 臂内同步 `await driver.reload_runtime()`：

- host 不进 tick → 无 status spinner，输入像冻死
- `reload_runtime` 非 cancel-safe：`reload.take()` 中途丢句柄；MCP 先卸旧 manager 再 connect

## Goals（已钉，与 proposal 一致）

| 项 | 钉死 |
|---|---|
| Status lead | `spinner + Reloading`（固定；无步骤轮换） |
| Host 态 | 独立 `reload_active`（名可微调）；**≠** `run_active` / bang busy |
| 软闸 | 可打字 / Ctrl+G；Enter 上行·slash·bang 拒 |
| 拒提 toast body | `reloading — wait` → 可见 `Error: reloading — wait` |
| Esc | overlay 先关 → 否则协作取消 reload → 再 idle Esc |
| Ctrl+C | `reload_active` 且无 overlay：**同 Esc 取消 reload**（不退出）；取消收口回到 idle 后，下一发 Ctrl+C 才走既有 idle 清输入/退出 |
| 取消语义 | 协作取消 + 一致收口；**不**事务回滚已写入的 skills/context |
| 失败/取消可见 | ScrollNotice 报告 **+** 壳层通告 |
| 超时 | 沿用 `MCP_SERVER_CONNECT_TIMEOUT` / `MCP_FIRST_TURN_GATE_TIMEOUT` |
| 跨面 | 仅 TUI |

## Approach

### 1. Host / effects 泵

```text
/reload (idle)
  → set reload_active, status=Reloading, soft-gate on
  → 非阻塞泵：keybindings（快）→ await reload_runtime(cancel)
       ↕ host 继续 tick / 打字 / Esc→cancel / Enter→toast
  → themes + catalog + loaded-resources 刷新
  → clear reload_active / Reloading / soft-gate
  → ScrollNotice 汇总；若 cancelled|failed → 另 toast
```

- **MUST NOT** 再在 `drain_pending` 里整段堵死 await（可 spawn/`select!`/分步 pending，实现自选，行为为准）。
- `reload_active` 期间：`is_busy()` 对 **agent busy-slash 表** 仍按「非 agent busy」理解——二次 `/reload` 由 **软闸** 拒绝，不是 `agent busy — /reload refused`。
- agent/bang 真 busy 时 `/reload` 仍 Reject（atm12 / 既有）。

### 2. Driver / MCP cancel-safe

今日坏序 → 目标序：

```text
snapshot = (tools_frozen_table, mcp_manager)
reopen / connect_and_discover（可 cancel）
  ok     → install new → freeze new → shutdown old
  cancel → restore snapshot（manager + freeze 表）→ 不 shutdown 仍在用的旧连接
  err    → 一致失败收口（builtins 或 snapshot，与今日部分成功报告对齐）+ shutdown 仅孤儿
任意出口 → put-back `self.reload`（禁止 take 后丢）
```

- 步骤间检查 cancel（skills 已跑完则保留，不回滚）。
- MCP connect 须能响应 cancel（token / `select!`），不得只靠「等 8s 超时」。

### 3. 文案常量（产品可见）

| 场景 | 落点 | 正文（body；toast 由渲染层加 `Error: `） |
|---|---|---|
| 软闸拒提交 | 壳层通告 | `reloading — wait` |
| 成功 | ScrollNotice | 既有 `Reload:\n…` 步进行 |
| 取消 | ScrollNotice | `Reload cancelled:` + 已完成步进摘要（格式对齐 `ok/failed` 行） |
| 取消 | 壳层通告 | `reload cancelled` |
| 失败（含超时诊断） | ScrollNotice | 既有 `Reload:` + `failed` 步进 |
| 失败 | 壳层通告 | `reload failed — see report` |

常量建议落 `commands.rs`（或等价 SSOT），与 `BUSY_SESSION_SWITCH_NOTICE` 同档。

### 4. Chrome

- `status.md`：busy 短词允许集增补 `Reloading`；并注明 **reload_active** 是第三种「类 busy」lead（非 agent / 非 bang）。
- `reload_active` 时 status **右侧**不画 mcp pending cue / next-turn cue。
- playground「Reloading」静图：**SHOULD**，非 apply 门禁。

### 5. Esc / Ctrl+C 优先级

```text
overlay 开 → 关槽（不 cancel reload）
else reload_active → 请求 cancel（Ctrl+C 同）
else → 既有 idle / agent / bang Esc 规则
```

取消请求已发出、尚未收口：重复 Esc/Ctrl+C **MUST NOT** 退出或开树；可忽略或保持取消中。

## Specs landing 意向（start 之后）

| Capability | 变更要点 |
|---|---|
| `app-tui-host` | 进行中 `Reloading`；软闸；取消收口；不堵 host tick；与 ath20/ath23 并存 |
| `app-tui-chrome` | status 词 / toast 文案；reload 时无右侧 cue |
| `app-tui-input` | reload_active 下 Ctrl+C/Esc 取消语义；可打字；Enter 拒 |
| `app-tui-commands` | atm12 补「进行中」行为（idle 启动后的 in-flight；agent busy 拒绝不变） |

Partitioned：toon 只加 requirements；可执行例进 `.feature` 或维持 atm12-unit harness（与现 slash-reload 一致时可 harness-only + `feature: false` 文档场景）。

## Risks

| 风险 | 缓解 |
|---|---|
| 取消半开 MCP | snapshot 换接；harness 取消用例断言 manager/tools 一致 |
| 软闸与 agent busy 文案混淆 | 独立 `reload_active`；拒提用 `reloading — wait` 非常量 busy 句 |
| 动画盖挂死 | 单 server / 门闸墙钟不变；失败必有 notice+toast |
| effects 泵复杂度 | 单一 drain 入口保留（ath6）；reload 臂可内嵌 select，禁止第二套 slash 匹配 |

## Verification

1. Harness：reload 进行中帧含 `Reloading`；可 `set_editor_text`；Enter → toast `reloading — wait`；无新 user 上行
2. Harness：注入慢 `reload_runtime` + Esc → `Reload cancelled` notice + toast `reload cancelled`；可再 `/reload`
3. Harness：注入失败 → notice 含 failed + toast `reload failed — see report`
4. 单测：MCP reload cancel 恢复 snapshot（旧 manager 仍可用 / 无泄漏路径可测则测）
5. 手测：真 MCP `/reload` 见 spinner、可打字、Esc 取消或等完成见汇总
