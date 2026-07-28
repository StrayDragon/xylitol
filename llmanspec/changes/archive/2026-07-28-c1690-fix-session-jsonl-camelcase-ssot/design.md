# Design: c1690-fix-session-jsonl-camelcase-ssot

## SSOT 决策（拍板）

| 面 | 命名 | 理由 |
|---|---|---|
| **Session JSONL（盘）** | **camelCase** | JSON 多端消费（含未来 TS web/app）；已有 `rename_all = "camelCase"`；避免盘↔前端双向 rename |
| Message role / part `type` | camelCase | 与盘内嵌套、LLM 方言一致 |
| Wire `Command` / `Event` | snake_case | Rust 服务端主流；**不进盘**；本 change 不改 |
| `XyEvent::event_name()` | snake | 订阅/日志键；与盘无关 |

**不是**「抄 pi」：pi 盘上 entry `type` 多为 snake；xylitol 盘 SSOT 是 **JS favor camel**。

## 现状漂移（实现须对齐）

| 漂移 | 目标 |
|---|---|
| `SESSION_VERSION=5` vs `create` 写 4 | 新写必须 5 |
| `s2` 写 snake 类型名 | 改为 camel 列表 |
| `alias = "bash_execution"` | **删除**；该行 skip+warn |
| `lift_bash_execution_entry` / 顶层 `BashExecution` 当上下文 | **删除读提升**；skip（或仅诊断，不进 LLM history） |
| `migrate_v3_to_v4` | **删除**；version < 5 整文件或行级按「非最新」skip/拒绝策略（见下） |
| 盘上 `parent_id` | 非 SSOT 键；不认作 `parentId`；行可解析但树链丢 → warn 或按坏行 |

## 坏行 / 旧栈策略

```text
load / 逐行 parse:
  parse JSON 失败 → skip + warn
  未知 type 或非 SSOT tag（含 bash_execution）→ skip + warn
  Header version ≠ SESSION_VERSION（5）→ 整文件拒绝或 header 失败 + warn
    （推荐：header 非 5 → load 失败返回可操作错误；条目行级旧形态 skip）
  message 内旧 untagged content → 该条 as_agent_message=None + skip（对齐 as46）
  warn 计数 > 3 → 第 4 条起合并为单条 "..."（不再刷屏）
list_sessions:
  单文件 get_name/load 失败 → 跳过该文件或占位，MUST NOT 失败整表
```

TUI `/session-resume`、print `--session`、CLI 恢复 **共用** SessionManager 策略，不在各面各写一套。

## 测试 seam

| Seam | 覆盖 |
|---|---|
| `SessionManager::create` / `append` / `load` | version=5；camel type；无顶层 bash |
| 黄金 / 拒绝 JSONL fixture | round-trip；snake tag skip；warn 封顶 |
| `list_sessions` | 一坏文件不拖垮 |
| 既有 `agent-session-store` / `agent-session` BDD | 更新断言中的 snake 类型名为 camel；去掉「lift 旧顶层 bash」期望 |

## 非目标

- 不改 wire camel/snake
- 不做盘文件批量 rewrite 工具
- 不挂 compaction OTel（c1700）
