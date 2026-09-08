---
depends_on: []
---

# 观测拆开书签 id 与 LLM 投影会话 id

## Why

今日 `langfuse.session.id` 只有一个值：xylitol 书签 UUID。分叉、前缀缓存、网关 session header 都挤在这个字段里，无法分析「这本对话」和「这次发给模型的会话身份」是不是同一回事。

已拍板：不兼容旧观测形状（无债）；数据结构要可扩展、好分析；**xylitol 书签 id** 与 **LLM API 投影会话 id** 必须拆开。树边（从哪本、哪个条目长出）一并进观测，否则分叉在 Langfuse 里是孤岛。

本刀在 A（并发槽）之后落地更稳，但提案先写下来以免遗忘。供应商键怎么算投影 id（缓存优先策略）是 B；C 先把**字段**钉死，B 未落地时投影 id 可暂时等于书签，但字段不得缺失。

## What Changes

- 观测属性拆成两套稳定键（名以实现为准，建议）：
  - 书签：`xylitol.session.bookmark_id`（用户能切换的那本）
  - 投影：`xylitol.session.llm_id`（发给模型 / 网关的会话身份）
- Langfuse `langfuse.session.id`：**等于书签**（产品 Session 视图 = 一本对话），不再偷偷改成投影 id
- 树边（fork 子本）：`xylitol.session.parent_bookmark_id`、`xylitol.session.fork_entry_id`（切点条目）；无父则省略
- 禁止再用单一 session.id 混指两套身份
- 不保留旧属性别名

## Capabilities

- `infra-otel`：新 req（建议 otel24）钉双 id + 树边；otel6 仍是 langfuse.session.id = 书签，并 MUST 同时写出 bookmark 显式键
- 可能触及 `package-ai-bridge` 观测快照结构（在 A 的 generate 快照上加字段）

## Impact

- Langfuse 里一本 xylitol 会话仍是一个 session；分析侧用 xylitol.* 看投影与分叉
- 旧 dashboard 若只认 langfuse.session.id，行为仍是书签（有意）
- 不改 fork 拷贝、不打开 prompt_cache_key

## Further Notes

切片链：A=`c2590-fix-obs-session-per-generate`（已归档）→ 本刀 → B=`c2620-add-provider-session-key-policy` 填满 `llm_id`。原伞 `c2580-add-session-identity-split` 已拆除，三切片全部物化。
