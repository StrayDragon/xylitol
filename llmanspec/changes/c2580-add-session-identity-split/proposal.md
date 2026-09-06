---
depends_on: []
---

# 会话三份身份：分叉缓存 × 观测槽 × Langfuse 树

## Why

分叉已是产品主线（会话树 travel / fork），但「会话」这个词同时指三件不同的事：

1. **书签**（bookmark）：用户能切换、命名、导出的那本对话（今日 JSONL 文件 id）。fork 会开一本新书签。
2. **树干前缀**（trunk prefix）：发给模型的稳定前缀。前缀匹配型 Prompt Cache（DeepSeek 磁盘前缀、llama.cpp 槽内 KV）吃的是这个。
3. **供应商会话键**（provider session key）：网关 header、`prompt_cache_key`、`previous_response_id` 这类「按会话路由/持态」的键。

今日实现把 **书签 id** 写进 Langfuse `langfuse.session.id`，并在 OpenCode Zen 上写成 `x-opencode-session`。Host 多槽并发时，观测上下文还是**进程一份** Mutex——后 bind 的会话会盖住先跑的那次请求的 session 属性。

若把「自由多重分叉」当基础能力，却继续用书签当供应商缓存键：分叉在产品上是共享树干，在供应商眼里却是新会话 → 冷缓存；观测上父/子书签断开，排障看不到「从哪条边长出来」。前缀匹配本身不是主风险；**把书签泄漏进供应商键 / 稿首**才是。

c2570 只保证「上游报了 cache 读数就不要假装 NotApplicable」。本草案管身份分离，不保证命中率。

## Open Questions

已拍板（2026-09-07）：

1. **缓存优先**：分叉后尽量让前缀/网关缓存仍有效，而不是「新书签 = 必冷」。供应商差异用**命名策略**分流（`compat` / WirePolicy 轮廓一类），**禁止**为「未来插件」新开 trait 市场。
2. **Langfuse 两套 id**：不兼容旧观测形状（无债）。`xylitol` **书签 id** 与 **LLM 投影/供应商会话 id** 必须拆开，各自可分析；不要用一个 `session.id` 混指。
3. **落地顺序**：先切片 A（观测槽按本次 generate / runtime，禁止进程槽竞态），再 B（供应商键策略），再 C（Langfuse 双 id + 树边）。

## What Changes

本 change 是 **purpose-draft / 探索壳**，建议拆成可独立归档的切片（见下），不要一次做完。

探索结论（代码事实，非愿望）：

- fork 今日是 **拷路径到新 JSONL**（新书签 id；条目 id 可与父本相同）。父文件不改。
- 模型稿里的 `session_env` 只有 date / clock / cwd，**没有**书签 UUID。`should_append_session_env` 只比 date/cwd——同日同目录 fork **不会**因 clock 再插一行（clock 刷新等于砍前缀）。
- `prompt_cache_key` / `previous_response_id` 产品默认关；DeepSeek 轮廓仍关。打开时若用书签 UUID 当 key，等于主动放弃共享树干。
- OpenCode Zen `x-opencode-session` = 当前书签（网关 courtesy）。fork 后子本会带新 header。DeepSeek / llama.cpp 不走此 header。
- Langfuse 会话 = 书签；**没有** parent_session / fork_at_entry。cache 读数已有三态（c2570）。
- `set_obs_session` 写进程槽；Host `SessionSlot` 可多本并发 `run_inflight`。测试已用 `ObsSessionScope` 避开槽污染，生产 HTTP 路径明确要求写进程槽。

## Recommended slices（认领时一次一块）

| 切片 | 用户可感知 | 规模 |
|---|---|---|
| **A. 观测槽按本次 generate** | 两本会话同时跑时，Langfuse / OpenCode header 不再张冠李戴 | 行为合约；须 SDD。**本刀落地**见 `c2590-fix-obs-session-per-generate` |
| **B. 供应商键策略（缓存优先）** | 分叉后前缀/网关缓存尽量仍命中；命名轮廓决定 header/`prompt_cache_key`，禁止插件式 trait 市场 | 跟在 A 之后 |
| **C. 观测双 id + 树边** | `xylitol` 书签 id 与 LLM 投影会话 id 拆开；parent/fork_at 可分析；无旧格式债 | 跟在 A 之后；会改 otel6「langfuse.session.id = 书签」的单一语义 |

## Capabilities

- `agent-session-store` / `agent-session`：fork 与书签语义（只读对照；本草案不改 live spec）
- `package-ai-bridge`：观测槽、attribution、WirePolicy 缓存旋钮
- `infra-otel` / 观测：Langfuse 属性
- `server-core`：Host 多槽并发

## Impact

- 产品：分叉仍是「新书签 + 共享历史心智」；缓存与观测不再偷偷绑死书签
- 供应商：前缀匹配族（官方 DeepSeek、本机 llama.cpp）与「按 session header 路由」族必须分开叙事
- 观测：先修槽，再加树 metadata

## Further Notes

- 口头锁（应升格进 architecture，已是代码事实）：**禁止稿首书签 UUID**；fork **保留拷来的 session_env**，不要因 clock 再插环境条。
- 打网（c2570）：DeepSeek / tufa 第二枝前缀命中；稿首 session id 或刷新 session_env clock → 命中打回 0。
- 调研笔记：`research/identities.md`（本目录）。
