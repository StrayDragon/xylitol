# Tasks — c1070-refactor-ai-bridge-sdk-projection

- [x] 1. Delta + design + tasks；`llman sdd validate c1070-refactor-ai-bridge-sdk-projection --no-interactive`
- [ ] 2. 定稿投影 API + 单测（bash/compact/branch/tool 行为）
- [ ] 3. 收窄 bridge DTO（去掉 session 平行角色）；删除 JSON roundtrip 全量 map
- [ ] 4. Completions → `async-openai` Client + middleware hooks
- [ ] 5. Responses + input_tokens → 同一 Client
- [ ] 6. Anthropic Messages + count_tokens → 官方 SDK（或经确认的兜底）
- [x] 7. 同步根/`src`/bridge `AGENTS.md`（规则已写入；实现落地后再跑满闸 qa）
- [ ] 8. `just qa` 相关子集全绿（随 apply）
