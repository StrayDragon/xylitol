---
change_id: c1550-add-otel-tool-observation-io
title: Langfuse tool.execute 可选 observation I/O 档
status: full
priority: 1550
depends_on:
- c1495-add-otel-session-span-tree
branch: feat/c1550-c1555-otel-observation-io
author: agent
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1550-add-otel-tool-observation-io

升格自 `do-not-read-me/c1540-add-otel-tool-observation-io`（原 c1540 号已被 timeout 变更占用，改用 c1550）。

## Why

c1495 后 Langfuse 能看到 `tool.execute` 树节点（name / tool_name），但 **input/output 恒空**。排障「调了什么参数、返回了什么」仍要回 TUI 或本地 JSONL。

用户期望：可选地把工具 I/O 挂到 observation 上，且**默认不上云**（路径、密钥、大段文件内容风险）。与 generation 的 `observation_io` **拆开配置**——LLM 与 tool 敏感度不同。

## What Changes

1. 配置：`[otel].tool_observation_io = none | truncated | full`（默认 **none**）
2. `none`：MUST NOT 写 `langfuse.observation.input` / `output` 到 `tool.execute`
3. `truncated` / `full`：在 tool span 结束前写入截断/尽量完整的参数 JSON 与结果摘要（硬顶对齐 generation：4096 / 65536 Unicode scalars）
4. 装配：组合根 → bridge 静态闸 `set_tool_observation_io_tier`；agent 经 bridge 闸写入，↛ infra

## Out of scope

- 默认开启 tool I/O
- `token.estimate` 双 root
- 修改 generation `observation_io` 语义（见 c1555 仅扩展 turn 根预览）

## Capabilities

- `infra-otel`（otel14 tool observation I/O 档）

## Impact

- 配置面多一个可选字段；默认行为不变
- Langfuse 在显式档下可读 tool 参数/结果

## Ethics

- risk_level: medium（敏感载荷上云）
- prohibited_actions: 默认 full；无截断硬顶的 full
- required_evidence: 默认 none 单测；truncated 有上限；validate
- escalation_policy: 与 `observation_io` 拆配置（非合并）
