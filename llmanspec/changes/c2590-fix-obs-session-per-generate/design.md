# Design: c2590 观测会话快照

## Decision

HTTP middleware 跑在 reqwest worker 上，TLS `ObsSessionScope` 过不了 hop（现注释已写明生产必须写进程槽）。进程槽在多 runtime 下会竞态。

权威源改为 **本次 generate 拷贝的 `ObsSessionContext`**：

- 放进 `AiBridgeGenerateOptions`（已有 `obs_parent`，同为「这次调用的观测信封」）
- Adapter 在组 HTTP 请求时把快照交给 hooks（per-call 包装，不靠全局）
- `llm.request` span 属性从同一快照写出
- 对齐 otel18：扇出前捕获，禁止全局槽竞态读写

进程槽：`set_obs_session` 可仍更新，供尚无 options 的闲置路径；重叠 generate MUST NOT 以它为权威。

## Options considered

| 选项 | 取舍 |
|---|---|
| A. Options + per-call hooks 包装 | 跨 hop 安全；改动面是 generate 入参 |
| B. 仅 tokio task-local | HTTP worker 读不到 |
| C. DashMap\<session, ctx\> | middleware 仍需知道「这是哪次请求」 |

选 A。不新开插件 trait。

## Non-goals

- Langfuse 双 id、树 parent/fork_at（C）
- `prompt_cache_key` / Zen header 改成树干 id（B）
- 删除进程槽（可后续）
