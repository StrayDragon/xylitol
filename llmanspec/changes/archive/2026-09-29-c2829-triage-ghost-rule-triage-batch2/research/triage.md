# 批 2 裁决记录（10 条 (c) naked）

| req | 裁决 | 证据 |
|---|---|---|
| r1122 | **转场景**（c2827 误判修正）| state_events 链路完整：tool_exec.rs:107-110 挂 tx、:201/:219 drain、react/mod.rs:1469 yield 进 run 流；场景 todo-update-emits-typed-event 证明回合流含类型化 TodoUpdated |
| r1447 | **转场景** | tests/support/mcp_fixture_server.py + connect_and_discover；场景 fixture-server-assembles-xytools |
| r1448 | **转场景**（装配随配置变化的可断言核心；热替换整链路留 reload 侧） | 两轮 discover 差集；场景 toolset-follows-config |
| r1449 | **转场景** | assemble.rs 既有语义：无效条目→diagnostics 非整体失败；场景 invalid-entry-diagnosed-not-fatal |
| r1462 | 留 naked → 批 3 | llm.request 仅 native HTTP 适配层导出，需 mock 流 harness |
| r1473 | 留 naked → 批 3 | 同上 |
| r1474 | 留 naked → 批 3 | 同上 |
| r1556 | 留 naked → 批 3 | HTTP 有界需可控慢上游 mock |
| r1415 | 留 naked → 批 3 | 地板诊断需「压后仍满窗」的满窗注入场景 |
| r1790 | 留 naked → 批 3 | session/resources 下行需 server 客户端循环缝 |

批 2 后 naked：10 → 6（全部归批 3）。
