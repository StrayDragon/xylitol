# Tasks

- [x] 1. **sdk**（session）：`WorkflowStep` 增加可选 `tools`（serde default）；旧 workflow 缺省为空。测：`cargo test -p xylitol-plugin-sdk workflow`。
- [x] 2. **workflow plugin**（middleware/tools）：`workflow_write` schema 与建计划契约声明 `tools`；`after_tool` 在步骤声明了 `tools` 时只对集合内工具失败标记 failed，未声明时回退粗归因，无关错误不改步骤状态。测：`cargo test -p xylitol-official-plugin-workflow`。
- [x] 3. **specs**（spec）：`agent-session-store` 的 workflow 数据形状补 `tools`；`agent-workflow` 的失败归因改为声明工具优先、旧 workflow 回退粗归因。测：`llman-sdd validate --all --strict`。
- [x] 4. **tests**（repo）：补 SDK `tools` 往返/缺省；中间件无关失败、匹配失败、旧 workflow 粗归因三条单测。测：`cargo test --all-features --test bdd`。
- [x] 5. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
