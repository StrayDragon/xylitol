# Tasks — c1000-add-infra-provider-trace

- [x] 1. `llman sdd validate c1000-add-infra-provider-trace --no-interactive`
- [x] 2. 依赖：加 `fastrace`（enable）+ `log` + 文件 logger；实现 file-only `Reporter`（禁 stderr）
- [x] 3. 改写 `app/cli/logging`：组合根 `set_reporter` + log 初始化；闸门 debug 默认 / release 开关；exit 路径 `flush`
- [x] 4. 全仓 `tracing::` → `log::`；移除 `tracing`/`tracing-subscriber`；`rg` 护栏
- [x] 5. `infra::provider::trace`：root span + raw/mapped Event；密钥禁入；三适配器接线
- [x] 6. 单测：gate off 无 dump；gate on 同 request_id 对照；TUI 不写 stdout/stderr
- [x] 7. 更新 `src/AGENTS.md` / 相关 AGENTS 指针；`validate --strict`
