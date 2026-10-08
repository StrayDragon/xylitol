# Tasks

## T1 — 有界决议
- [done] `mcp.rs ensure_tool_table_frozen` pub 化。
- [done] host.rs METHOD_ARM_TOOL_FREEZE handler 改用 `ensure_tool_table_frozen`（注释 c2847）。
- 校验：`arm_tool_freeze_unary_freezes_after_mcp_settle` 仍绿。

## T2 — 回归测试
- [done] `arm_tool_freeze_hangs_mcp_returns_frozen_within_gate_window`（挂死 MCP → 有界返回 + frozen+complete）。
- 校验：单测绿。

## T3 — spec + 门禁收口
- [done] server-core r1923 规则 + verified-by 锚向回归测试（无场景，单测为自动化锁）。
- [done] fmt / clippy 0 / lib 1620 / BDD 957/957 / validate 0。
- 收口：finalize c2847 + push。
