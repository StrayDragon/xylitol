---
depends_on: []
branch: sdd/c2590-fix-obs-session-per-generate
base_sha: 5c10e3b0c2afd38c6dfbffb92b56ee142f5fb080
checkpointed: false
---

# 观测会话上下文跟本次 generate，禁止进程槽竞态

## Why

Host 可多槽并行 `run_inflight`。`set_obs_session` 写进程 Mutex，OpenAI HTTP middleware 与 Langfuse 属性事后再读全局槽。后 bind 的会话会盖住先发出、尚未结束的请求：`langfuse.session.id` 与 OpenCode `x-opencode-session` 张冠李戴。otel18 已禁止工具 span 依赖全局 parent slot；会话 id 仍走全局槽。切片 A 先修这个，否则分叉缓存与双 id 观测（c2580 B/C）证据不可信。

本刀 **不** 改「langfuse.session.id = xylitol 书签 UUID」的 otel6 语义，只保证并发时是**那一次处理所绑定的书签**。书签 vs LLM 投影会话 id 拆分留给 C。

## What Changes

- 每次 `generate` / `llm.request` 携带一份观测会话快照（从该 runtime 绑定的书签拷出）
- HTTP hooks / attribution / generation span 读这份快照，MUST NOT 读可被其它 runtime 覆盖的进程槽
- 进程槽可留作无 options 的闲置路径回退，但 MUST NOT 作为重叠 generate 的权威源
- 单测：两路重叠 generate、不同书签，header 与 span 不错位

## Capabilities

- `infra-otel`：otel23 并发会话 id 不错位；otel6 仍是书签 UUID，主语改为「该次处理所绑定」
- `package-ai-bridge`：pab30 generate 快照驱动 attribution / generation 观测

## Impact

- 两本会话同时打网关时，Langfuse 与 Zen header 各跟各的书签
- 不打开 `prompt_cache_key`；不改 fork 拷贝；不引入插件 trait

## Further Notes

伞形探索：`c2580-add-session-identity-split`。B = 供应商键策略（缓存优先、命名轮廓）；C = 观测双 id + 树边。
