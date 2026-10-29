# Tasks

- [x] 1. **sdk**（settlement）：`ContextTokenSettlement` 与 `XyEvent::ContextTokenSettlement` 增加 `prompt_cache_read: PromptCacheRead`；turn_end / compact_ops 从最近一次 usage 填充，无 usage 保持 `NotApplicable`。测：`cargo test -p xylitol-plugin-sdk settlement`。
- [x] 2. **protocol / wire**（blocked-by: 1）：`Event::ContextTokenSettlement` 增加 `prompt_cache` / `prompt_cache_tokens`；`xylitol-wire` 做 SDK enum ↔ protocol 字符串映射，未知按 `unknown`。测：`cargo test -p xylitol-protocol`、`cargo test -p xylitol-wire`。
- [x] 3. **app-tui**（blocked-by: 2）：footer 在 `used` 后渲染 `· cache N` / `· cache ?`，`NotApplicable` 省略；host 刷新走 settlement。测：`cargo test -p xylitol-app-tui`。
- [x] 4. **obs**（blocked-by: 2）：校验 OpenAI / Anthropic 两族 provider trace 都发 `xylitol.prompt_cache_read` 三态与 `usage_details.cache_read`（仅 Tokens）；补一致性单测。不新增本地观测 CLI。测：`cargo test -p xylitol-official-plugin-provider-openai`、`cargo test -p xylitol-official-plugin-provider-anthropic`。
- [x] 5. **specs**（app-tui-fixed-zone / protocol-app / server-core）：扩展 `r1224` footer 字段、`r1698` settlement 载荷；补三态场景。测：`llman-sdd validate c2994-update-cache-usage-surface`。
- [x] 6. **tests**（repo）：protocol 往返、wire 三态映射、footer 三态、obs fixture。测：`cargo test --all-features --test bdd`。
- [x] 7. **门禁**（repo）：`just qa` 全绿；收口 PR body 含 `fixed #22`。测：`just qa`。
