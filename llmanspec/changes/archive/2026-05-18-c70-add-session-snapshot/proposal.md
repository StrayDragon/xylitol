---
depends_on: [c10-add-config, c25-add-agent-loop, c45-add-lsp-layer]
---

# c70-add-session-snapshot

## Why

Session 快照系统使 agent 拥有持久化状态、上下文派生、蜂群协作的能力。快照是不可变的一等公民，包含对话历史、代码认知、工具调用摘要（§11）。

## What Changes

1. 在 `src/infra/session/` 实现快照系统
2. 核心操作：snapshot / restore / spawn / list / prune / fine-tune / diff / merge
3. 数据结构：对话、项目认知（code_summaries, codebase_graph）、调试器状态、工具调用日志
4. 上下文压缩（compaction）：对话摘要 + 工具调用折叠
5. 增量快照（CoW）+ Zstandard 压缩 + GC 策略
6. 蜂群协作：从快照派生新 agent 实例

> **⏸️ DAP PAUSED** — `debugger_state`（DAP 产物）字段已暂停（2026-05-17）。DAP 开发恢复前此字段为 `None`/空。

### 快照数据结构

```yaml
snapshot:
  id, created, parent_snapshot_id
  conversation: [...]          # 完整多轮对话
  project_cognition:
    code_summaries: {...}      # LSP 提炼的模块摘要
    codebase_graph: {...}      # 依赖关系
    debugger_state: {...}      # DAP 产物
  tool_call_log: [...]         # 仅摘要
  config_fingerprint: {...}
```

### Compaction 策略

| 触发条件 | 策略 |
|---------|------|
| 对话 token > 75% 窗口 | intra_compaction（自动摘要） |
| 用户显式请求 | manual_compaction |
| 快照派生时 | derive_compaction |

## Capabilities

- `session-snapshot`: 快照创建/派生/合并 + 上下文压缩 + 蜂群协作

## Impact

- 新增 `adk-session`（SQLite 后端）、`zstd` 依赖
- feature flag `infra-session` 启用此模块
- 依赖 c45 LSP 层提供 `code_summaries`
