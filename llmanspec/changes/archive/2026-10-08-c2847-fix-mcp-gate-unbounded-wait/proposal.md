---
depends_on: []
needs_specs_change: true
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# 修复 arm_tool_freeze 无界等待（MCP 配置不可达时首轮死锁）

## Why

serve 配置了 MCP（context7/lspz）但**连不上**（SSE 无网 / stdio 挂死）时，MCP bootstrap 永驻 `connecting`，`mcp_bootstrap_complete:false`。TUI 首轮「Assembling」门等待 `tools_table_frozen` 翻转，而真正的超时 detach-freeze 只在 **runtime `run()`** 里 `ensure_tool_table_frozen` 触发——但 run 又等门开。**双向死锁**：TUI 永不 run、run 永不启动 → 追加消息永远卡在工作 spinner、provider 请求从不发出（用户实机复现：langfuse 只见旧请求）。

## What Changes

1. **`arm_tool_freeze` unary handler**（`src/app/server/host.rs`）：从「arm + 单次 poll 即回快照」改为调用 `ensure_tool_table_frozen()`（既有、有界：`MCP_FIRST_TURN_GATE_TIMEOUT=15s` 超时 detach 未连上的 bootstrap 并冻结 armed 子集）后再返回权威快照。返回快照 `tools_table_frozen=true`、`mcp_bootstrap_complete=true` → TUI 门在时限内有界打开 → turn 照跑（工具降级 absent + gate notice）。
2. **可见性**：`mcp.rs ensure_tool_table_frozen` `pub(super)` → `pub`（host.rs 在 server 层，需可见）。
3. **回归测试**（`remote.rs` `arm_tool_freeze_hangs_mcp_returns_frozen_within_gate_window`）：挂死 MCP（`sleep 300`）→ unary 在 30s 内有界返回且快照 frozen+complete（旧行为：立即返回未冻结 → 客户端无限等）。
4. **spec `server-core` r1923** + 场景锁定（有界决议）。

## Capabilities

- `app/server`（unary 权威决议）、`agent`（首轮门机制复用）

## Impact / 风险

- unary 最多阻塞门时限（15s+2s 边张）；该窗口内仅等待中的首轮客户端受影响（与会话槽绑定，其他槽不受影响；与 runtime 自己的行为一致）。
- 产品语义不变：门时限内正常 MCP settle 仍等待；超时才降级放行。
