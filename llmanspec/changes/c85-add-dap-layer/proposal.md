---
depends_on: [c10-add-config]
status: paused
paused_reason: "DAP 集成暂停开发"
paused_date: 2026-05-17
---

# c85-add-dap-layer

> **⏸️ PAUSED** — DAP 集成已暂停（2026-05-17）。此 change 所有内容均不实施，`infra-dap` feature flag 保留但无功能逻辑。如需恢复，请更新 `status` 为 `active` 并更新设计文档。

## Why

DAP 调试器集成的架构预留。dapz v0.0 处于极早期，暂不实际集成，仅预留模块结构和配置入口（§4）。

## What Changes

1. 在 `src/infra/dap/` 定义 DAP 兼容层接口（trait + 数据结构）
2. 配置入口（YAML 中 `dap` 段）
3. 不实际集成任何 DAP 后端

### 架构预留

```rust
trait DapClient {
    async fn attach(&self, program: &str) -> Result<DapSession>;
    async fn set_breakpoints(&self, file: &str, lines: &[u32]) -> Result<()>;
    async fn continue_execution(&self) -> Result<DapEvent>;
    async fn get_variables(&self, scope: VariableScope) -> Result<Vec<Variable>>;
    async fn get_stack_trace(&self) -> Result<Vec<StackFrame>>;
    async fn disconnect(&self) -> Result<()>;
}
```

### YAML 配置

```yaml
dap:
  enabled: false
  backends:
    rust: "lldb-dap"
    python: "debugpy"
    typescript: "vscode-js-debug"
```

## Capabilities

- `dap-layer`: DAP 调试器集成架构预留（无实际功能）

## Impact

- 仅占位代码，无外部依赖
- feature flag `infra-dap` 启用（但为空实现）
- 后期独立阶段交付
