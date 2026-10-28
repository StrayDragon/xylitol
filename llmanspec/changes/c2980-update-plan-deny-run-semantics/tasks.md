# Tasks

- [ ] 1. **sdk**（plugin-sdk）：`BeforeToolCtx` 增加 `deny_reason: Option<String>`；`SDK_API_VERSION` bump 到 `0.6.0`。测：`cargo test -p xylitol-plugin-sdk`。
- [ ] 2. **runtime**（tool_exec）：`prepare_call` 读取链终态 `deny_reason`，置 `denied_reason` 但不置 `stop_run`；工具不执行。测：`cargo test -p xylitol-runtime agent::runtime::tool_exec::tests`。
- [ ] 3. **plan plugin**（middleware）：只读闸拒绝时设置 `deny_reason` 并返回 `Continue`（仍保留观测属性与 args 标注）。测：`cargo test -p xylitol-official-plugin-plan`。
- [ ] 4. **specs**（spec）：`agent-plan` 的 `r2960` 明确拒绝不结束本 run、模型可同回合改调 allowlist 工具；新增「拒绝写入后本回合继续」场景。测：`llman-sdd validate --all --strict`。
- [ ] 5. **tests**（repo）：补 runtime deny 不终止 run 单测、plan 中间件 Continue + deny_reason 断言、BDD 新场景绑定与步骤。测：`cargo test --all-features --test bdd`。
- [ ] 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
