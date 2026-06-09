---
depends_on: []
---
# Proposal: c05-rebuild-core

## Why

Xylitol 当前核心（tools、session、agent loop、config、hooks）与 pi coding-agent 存在
显著行为差异。本项目目标是 100% 复刻 pi 的 core 行为（不含 TUI 和 Extensions SDK），
同时保留 Rust 的编译期安全和性能优势。

采用方案 A：一个巨大 change 一次性整体重写所有核心模块，
避免增量 change 的桥接/适配开销。

## What Changes

### 重写的模块（完全对齐 pi）

| 模块 | pi 对应 | 关键行为 |
|------|---------|---------|
| **Tool System** (7 tools) | `packages/coding-agent/src/core/tools/` | multi-edit、ripgrep grep、fd find、truncation、pluggable ops、mutation queue、abort |
| **Session Persistence** | `session-manager.ts` | JSONL 文件存储、Branching、Tree navigation |
| **Agent Session** | `agent-session.ts` | 完整生命周期、turn events、model 切换、thinking level toggle |
| **Agent Loop** | agent-core package | 事件流 (turn_start/end, message_start/update/end, tool_execution_start/update/end) |
| **Compaction** | `compaction/` | context window 检测与压缩 |
| **CLI Entry** | `cli/args.ts` | pi 完整 CLI args |
| **Config** | `config.ts` + SettingsManager | YAML 三层配置 (global/project/user) |
| **Hook System** | extension events (精简版) | 升级 hook 点以支持 prefix-caching 特化（如 deepseek v4 的 cache_control） |

### 移除的模块

| 模块 | 原因 |
|------|------|
| security-policy | 过度设计，pi 无对应物，用更轻量的方案替代 |
| repeat-guard | 由 agent loop 的 compaction 自然处理 |
| planning-orchestrator | pi 无 plan mode，由 agent + skills 替代 |

### 保留不变

| 模块 | 原因 |
|------|------|
| diff-review、tui-interface、markdown-rendering、print-output | 后续 TUI change |
| acp-protocol、lsp-integration、skill-extension、fake-provider | 后续 change |
| build-config、workspace-structure、test-infra | 基础设施 |

## Capabilities

- `tool-system`: 完全重写
- `session-persistence`: 完全重写
- `agent-session`: 新建
- `agent-runtime`: 重写
- `compaction`: 新建
- `cli-entry`: 更新
- `runtime-config`: 重写
- `hook-system`: 升级
- `security-policy`: **移除**
- `repeat-guard`: **移除**
- `planning-orchestrator`: **移除**

## Impact

- **破坏性变更**：整个 `src/agent/`、`src/infra/session/`、`src/infra/config/`、`src/infra/hooks/` 重写
- **新依赖**：无（grep/find 通过外部 rg/fd 进程调用）
- **删除依赖**：无
- **BDD 驱动**：全部通过 `.feature` 文件定义行为，实现全部 step
