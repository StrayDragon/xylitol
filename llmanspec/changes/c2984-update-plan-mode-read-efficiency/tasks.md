# Tasks

- [ ] 1. **sdk**（plugin-sdk）：`BeforeLlmRequestCtx` 增加 `plan_mode: Option<PlanMode>`；`SDK_API_VERSION` bump 到 `0.5.0`。测：`cargo test -p xylitol-plugin-sdk`。
- [ ] 2. **runtime**（react）：每迭代只读一次 leaf branch，把 `latest_plan_mode` 与状态栏共用同一分支结果，经 ctx 传给中间件。测：`cargo test -p xylitol-runtime`。
- [ ] 3. **plan plugin**（middleware）：`before_llm_request` 直接消费 `ctx.plan_mode`，删除 `load_mode` / store / ctx 字段；缺失时降级为 execute。测：`cargo test -p xylitol-official-plugin-plan`。
- [ ] 4. **tests**（repo）：更新所有 `BeforeLlmRequestCtx` 构造点；补「缺失 plan_mode 降级 execute」单测；BDD plan gate 改由 ctx 注入模式。测：`cargo test --all-features --test bdd`。
- [ ] 5. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
