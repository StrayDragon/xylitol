---
depends_on: []
branch: sdd/c2829-triage-ghost-rule-triage-batch2
base_branch: main
base_sha: 766e44a4f59a919f1752c303df2d3560bf3ddb04
---

## Why

批 2（幽灵规则裁决）：对 c2828 后剩余 10 条 naked (c) 规则逐条核对代码。侦察修正一处误判——r1122 的 state_events 链路**完整**（tool_exec.rs:107-110 挂 tx、:201/:219 drain 进 run 流），可转场景；MCP fixture（tests/support/mcp_fixture_server.py + connect_and_discover）机构齐全，r1447–1449 可转场景。

## What Changes

- r1122 转场景：agent 回合内 todo_update 成功 → run 流含类型化 TodoUpdated 全量快照。
- r1447 转场景：fixture MCP server 经 connect_and_discover 装配出 mcp__ 前缀 XyTool。
- r1448 转场景：装配随配置变化（不同 XYLITOL_MCP_FIXTURE_TOOLS → 不同工具集）。
- r1449 转场景：无效条目不整体失败，留在 diagnostics 且工具集为空。
- r1462/r1473/r1474/r1556/r1415/r1790 维持 naked（缺 native HTTP mock / 满窗注入 / server 下行缝，归批 3），evidence 记录于 research/triage.md。

## Scope

- `llmanspec/specs/**`（4 条规则加场景）
- `tests/bdd/steps_c2827.rs`、`tests/bdd/bindings_c2827.rs`（新步骤与绑定）

## Out of Scope

- 不改产品代码（r1122 链路已存在，纯补可执行场景）。
