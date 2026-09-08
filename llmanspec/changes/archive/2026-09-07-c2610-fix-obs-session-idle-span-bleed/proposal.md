---
depends_on: []
branch: sdd/c2610-fix-obs-session-idle-span-bleed
base_sha: 4ab4e5cbcd96e39ee7d3101e103482d85a8fc995
checkpointed: true
checkpoint_sha: 4ab4e5cbcd96e39ee7d3101e103482d85a8fc995
---

# 观测会话 id 全树一致：idle 路径 span 收编进快照，reader 物化禁止踩槽

## Why

c2590 / otel23 只把 `agent.turn` 根与 `llm.request` 收进 per-generate 快照；`agent.iteration`、`tool.execute`、`react.error` / `tool.error`、`agent.compaction` 以及 compaction summarizer 的 `llm.request` 仍在 span 创建时读**进程级观测槽**（`obs_session.rs` 的全局 Mutex）。host 按 session 维护多槽（`HashMap<String, SessionSlot>`），且 **reader 物化**（stats / tree / messages 等非 writer RPC 的临时 driver）也走 `switch_session → set_session → set_obs_session` 写同一全局槽：新会话 generate 期间任何触碰其它会话的操作都会把槽踩回那个会话的 id。同一 trace 内于是混出两种 `langfuse.session.id`；Langfuse 把任意 span 上的该属性提升为**整 trace** 属性且要求各 span 一致（冲突语义未定义），实测表现为新会话整棵 trace 被归进旧 session。

## What Changes

- 单次处理导出的低频观测 span（`agent.turn` / `agent.iteration` / `llm.request` / `tool.execute` / 过 prepare 的 `agent.compaction` / `token.estimate` / `react.error` / `tool.error`，含 compaction summarizer 发起的 `llm.request`）的 `langfuse.session.id` MUST 全部来自该次处理绑定的会话快照；重叠处理下 MUST NOT 读进程槽
- host 的 reader / 只读物化 MUST NOT 写观测槽；槽保留为 writer bind 的闲置回退（otel23 语义不变）
- 不改 otel6「值为书签 UUID」语义；`session_name` 快照化与双身份拆分留给 c2600（本 change 是其前置去噪）

## Capabilities

- `infra-otel`：otel24 单次处理全树会话 id 快照一致；otel25 观测槽写入纪律（仅 writer 绑定路径）

## Impact

- 新会话 trace 不再落入旧 session；多槽重叠 generate 各树自洽
- react / compaction 调用链窄改透传快照；无 run 上下文的 slash compaction 允许槽回退
