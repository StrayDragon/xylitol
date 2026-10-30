# 设计：OpenAI 缓存亲和

## 载体

- 缓存开关与 retention 通过 `WirePolicy.extra_policy` 承载：`prompt_cache_key: bool` 已有，新增 `prompt_cache_retention: Option<XyPromptCacheRetention>`。
- `resolve_wire_policy`（内核工厂）在 `WirePolicy::for_compat` 之后应用配置覆盖，避免给 assembler / 适配器增加参数。

## 默认值

- `PROMPT_CACHE_KEY` 由 `false` 改为 `true`（generic）；`for_compat(Deepseek)` 保持 `false`。
- retention 缺省 `None`（省略字段，官方默认 `in_memory`）。

## 发送逻辑

- Responses：在 `assemble_responses_body_with_diagnostics` 内，当 `allows_prompt_cache_key()` 且 `obs_session.session_id` 非空时写 `prompt_cache_key`；retention 仅在有 key 时写。
- Completions：序列化后按同一条件注入；流式与非流式共用 helper。

## 风险

- 会话 id 可能含非 ASCII 或超长；官方未给上限，上游实现截断到 64 字符。本切片按原样发送，不引入截断。
- 开启后不同会话不再共享缓存路由；符合「同会话亲和」意图。
