---
depends_on: [c10-add-config]
---

# c45-add-lsp-layer

## Why

LSP Token 节省层是本项目独有的能力（其他 agent 均无），通过 lspz agent-sdk 内嵌集成，为 agent 提供代码智能（诊断、补全、定义跳转等）并压缩输出（§3）。

## What Changes

1. 在 `src/infra/lsp/` 封装 lspz `agent-sdk` feature
2. 三种初始化策略：懒加载（推荐）/ 预初始化 / 按需预热
3. `AgentPool` 管理 LSP server 子进程生命周期
4. 10+ 查询方法 + 3 个文件同步方法 + 3 个重构操作
5. TOON 文本压缩输出（直接拼入 LLM prompt）

### 集成模式

Agent 二进制直接 spawn 并管理 LSP server 子进程（通过 `AgentHandle` / `AgentPool`），无需外部 proxy。

### 返回格式

查询结果返回压缩后的 TOON 文本（`String`），直接拼入 agent prompt。

## Capabilities

- `lsp-layer`: lspz agent-sdk 封装 + 子进程管理 + 压缩查询

## Impact

- 新增 `lspz` 依赖（feature = "agent-sdk"）
- feature flag `infra-lsp` 启用此模块
- LSP server 子进程生命周期管理
