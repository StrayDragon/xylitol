# Tasks

- [ ] 1. **measure**（kernel）：一次性探针统计默认工具表 schema 字符/粗略 token；结果写入 `design.md`（plan 全组约 27.6%，`plan_write+plan_submit` 约 15.9%）。
- [ ] 2. **plan plugin**（middleware）：新增 `PlanToolSurfaceMiddleware`，execute 模式下从模型可见 `tool_schemas` 移除 `plan_write` / `plan_submit`，保留 `plan_read` / `plan_list` / `plan_mode`；plan 模式不改。插件注册/卸载成对管理。
- [ ] 3. **specs**（spec）：`agent-plan` 新增 `r2974` 工具面规则（execute 隐藏写/提交，其余常驻；只影响可见 schema，不改权限与只读闸）。测：`llman-sdd validate --all --strict`。
- [ ] 4. **tests**（plan）：补 execute/plan/缺失模式三组工具面单测；插件注册 id 断言含 `plan-tool-surface`。测：`cargo test -p xylitol-official-plugin-plan`。
- [ ] 5. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
