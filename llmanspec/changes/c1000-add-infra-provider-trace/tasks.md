# Tasks — c1000-add-infra-provider-trace

- [x] 1. `llman sdd validate c1000-add-infra-provider-trace --no-interactive`
- [ ] 2. 落地 `infra::provider::trace` helper（request span、raw/mapped emit、enabled 门闩、密钥头禁入）
- [ ] 3. logging 组合根：专用 `provider-trace.jsonl` Layer；闸门 debug 默认 / release 开关
- [ ] 4. Responses + Anthropic + Completions 适配器接线（同 request_id 双记）
- [ ] 5. 单测：gate off 无文件或无可观测行；gate on 有 raw+mapped 对照
- [ ] 6. 更新 `src/AGENTS.md` 一行指针；`validate --strict`
