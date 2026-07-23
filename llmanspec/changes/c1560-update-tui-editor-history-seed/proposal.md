---
change_id: c1560-update-tui-editor-history-seed
title: 新 session 从最近 N 个同 cwd session 种子 ↑/↓ 用户输入；恢复/切换仅装当前 session
status: purpose-draft
priority: 1560
depends_on: []
author: agent
---

# c1560-update-tui-editor-history-seed

> **阶段**：purpose-draft。由原「一批 UX」草案拆出；**正交**于 c1565（CLI/resume 提示）、c1570（Ctrl+C）。
> **决策已收敛** — Open Questions 已清空。

## Why

↑/↓ 发送历史今天只活在当前进程（`ati13`）。新开 session 时用户常想复用上一场已打过的 prompt，却要 `/session-resume` 回旧会话或重打。

## 代码事实

- `Editor::add_to_history`（newest@0，上限 100）；产品 `remember_editor_send`
- `XyDriver::list_sessions`（mtime 降序）+ `load_entries` 已可抽 user 正文；条目含 `cwd`
- bootstrap 总是分配新 Uuid；落盘在 `ensure_session` / 首条消息之后

## Decisions（已锁）

### D1. 种子策略

| 入口 | ↑/↓ 种子 |
|---|---|
| **纯 new session**（未 CLI 恢复、未切到已有 id；含 `/session-new`） | 在 **当前 cwd** 的已持久化 session 中，按 mtime 取最近 **N** 个**其它** session（默认 **N=1**，可配置），抽取 **user** 正文，时间序 `add_to_history`（↑ 先到最近一条） |
| **恢复 / 切换到已有 session**（`--session`、`/session-resume`、落到已有 id 等） | **仅**该 session 的 user 正文；**替换**当前 editor 发送历史缓冲 |

- **MUST NOT** 把 assistant / tool / thinking 写入发送历史
- **MUST NOT** 改 transcript / 会话树内容
- 配置键（promote 时定名）：建议 `tui.editor_history_seed_sessions`，默认 `1`
- 放宽 `ati13`：允许启动/切 session 时从 store **只读种子**；仍不要求把 editor 浏览态做成独立持久化文件

### D2. 「最近 N 个」范围

**仅当前 cwd**（与 resume 面板 Current scope 同口径；cwd 缺失或不匹配的 session 不计入 N）。

## What Changes（意向）

- Host：启动 / switch_session / session-new 后装载种子
- `runtime-config` + harness：new vs resume；cwd 过滤用例
- 修订 `app-tui-input` `ati13`（及必要时 host/session specs）

## Non-Goals

- 全局跨机器 prompt 史文件
- CLI 旗标形状、退出文案、Ctrl+C（见 c1565 / c1570）

## Open Questions

（已清空。）

## Related

- `app-tui-input`（`ati13`）、`runtime-config`、`agent-session-store`（只读）
- 设计：`src/app/tui/design/editor.md`
- 并行 draft：`c1565`、`c1570`

## Ethics

- `ethics.risk_level`: low
- `ethics.prohibited_actions`: draft 阶段写应用代码
- `ethics.required_evidence`: harness new/resume 种子 + cwd 过滤用例
- `ethics.escalation_policy`: 无

## Next

现有 SDD 流水线告一段落后再 `/llman-sdd-propose` promote → attach → apply（可与 c1565/c1570 并行）
