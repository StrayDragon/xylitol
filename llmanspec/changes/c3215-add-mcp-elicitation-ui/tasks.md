# Tasks — MCP elicitation form UI

- [ ] 1. schema：校验 / 渲染 String / Number / Integer / Boolean / Enum；title / description / default / required；string format / min / max；未知形状 decline。
- [ ] 2. 秘密策略：字段名命中 password / secret / token / api_key / credential / private_key → 整表 decline；URL 模式仍 decline。
- [ ] 3. Host / TUI：elicitation 请求事件、表单槽位、响应 unary；5 分钟超时 decline；无 TUI 按 decline。
- [ ] 4. MRTR：accept 构造 `ElicitResult { action: Accept, content }`；类型/必填校验；decline/cancel 不带 content。
- [ ] 5. 审计：server / 字段名 / action / reason / timestamp；不写值 / message / schema。
- [ ] 6. Specs：`infra-mcp` elicitation UI 规则；单测 / BDD。
- [ ] 7. 门禁：`just fmt` / `just lint` / `just qa` / `just qa-e2e`。
