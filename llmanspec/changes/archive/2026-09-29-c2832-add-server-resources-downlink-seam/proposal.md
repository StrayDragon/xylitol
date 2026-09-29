---
depends_on: []
branch: sdd/c2832-add-server-resources-downlink-seam
base_branch: main
base_sha: 4602fd0d5385c25860fbf8b7ae58d692a4dada53
---

## Why

批 3 遗留：r1790（session/resources 下行推送）判 (c) missing-surface。核查发现实现与近邻测试已齐：`push_resources` 广播 + `ensure_mcp_resources_watch`（50ms poll → 变化才推）在 host.rs，`push_resources_is_not_journaled` 已钉 payload 形状与 seq 不消耗，客户端未知方法忽略由 remote.rs dispatch 臂承载。**唯一缺口是 watch 循环端到端**：fake resources provider（fixture MCP server）驱动快照变化 → 订阅客户端恰收一帧。

## What Changes

- remote.rs 测试模块新增 `resources_watch_pushes_one_frame_on_snapshot_change`：`for_test_with_mcp` + fixture MCP server + `materialize_writer` + `in_process_downlink` 订阅 + `ensure_mcp_resources_watch`。
  - 断言一：bootstrap 落定后恰收一帧 `session/resources`（20s 内）。
  - 断言二：payload.snapshot 与 `loaded_resources_snapshot_for` 逐字段同形（unary 一致性）。
  - 断言三：journal `max_seq` 不动（notification 不消耗 seq）。
  - 断言四：快照稳定后 700ms 静默窗口无重复帧（仅变化时推）。
- r1790 加 `# verified-by: fn resources_watch_pushes_one_frame_on_snapshot_change` 锚。

## Evidence

- llmanspec/specs/server-core r1790；host.rs push_resources / ensure_mcp_resources_watch；remote.rs:2065 push_resources_is_not_journaled、:2382 fixture_mcp 模式。
- 矩阵预期：with-anchor 407→408、naked 1→0（naked 归零，核对阶段收口）。
