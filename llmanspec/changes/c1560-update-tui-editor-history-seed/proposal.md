---
change_id: c1560-update-tui-editor-history-seed
title: 新 session 从最近 N 个同 cwd session 种子 ↑/↓ 用户输入；恢复/切换仅装当前 session
status: in-progress
priority: 1560
depends_on: []
author: agent
branch: feat/c1560-tui-editor-history-seed
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: false
---

# c1560-update-tui-editor-history-seed

> **full 工件已齐**（proposal + design + tasks + live specs）。决策已锁。下一步：`attach` → `llman-sdd-apply`。

## Why

↑/↓ 发送历史今天只活在当前进程（`ati13`）。新开 session 时用户常想复用上一场已打过的 prompt，却要 `/session-resume` 回旧会话或重打。

## 代码事实

- `Editor::add_to_history`（newest@0，上限 100）；产品 `remember_editor_send`
- `XyDriver::list_sessions`（mtime 降序）+ `load_entries`；`cwd` + resume `cwd_matches`
- bootstrap 总分配 Uuid；落盘在 `ensure_session` / 首条消息之后

## Decisions（已锁）

### D1. 种子策略

| 入口 | ↑/↓ 种子 |
|---|---|
| **纯 new session**（未 CLI 恢复、未切到已有 id；含 `/session-new`） | 当前 **cwd** 下按 mtime 最近 **N** 个其它已持久化 session（默认 **N=1**），抽 **user** 正文，时间序 `add_to_history` |
| **恢复 / 切换到已有 session** | **仅**该 session 的 user 正文；**替换** editor 发送历史缓冲 |

- **MUST NOT** 写入 assistant / tool / thinking；**MUST NOT** 改 transcript / 会话树
- 配置键：**`tui.editor_history_seed_sessions`**，默认 **`1`**
- 种子正文：**跳过** trim 后以 `/` 开头的 user 行
- N>1：较旧 session 整段先种，再较新；↑ 先到全局最近一条
- cwd：复用 `cwd_matches`
- 放宽 `ati13`：允许启动/切 session 时从 store **只读种子**

### D2. 范围

仅当前 cwd（cwd 缺失/不匹配不计入 N）。

## Capabilities

- `app-tui-input`（改 `ati13` + 新 req）
- `runtime-config`（配置键）

## Impact

产品 TUI Host 启动与 session 切换路径；配置 schema；harness。

## Non-Goals

- CLI 旗标 / resume 文案 / Ctrl+C / busy slash / queue drain（c1565/c1570/c1580/c1585）

## Open Questions

（已清空。）

## Related

- `c1565`、`c1570`、`c1580`、`c1585`（正交延后）

## Ethics

- `ethics.risk_level`: low
- `ethics.prohibited_actions`: 未 attach 前在 main 改 live MUST 并实现
- `ethics.required_evidence`: harness new/resume + cwd 过滤
- `ethics.escalation_policy`: 无
