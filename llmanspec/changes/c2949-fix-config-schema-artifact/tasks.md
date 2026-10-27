# 任务

- [ ] 1. **生成器**（config 插件）：新增 `gen-config-schema` bin，输出 `schema_for!(AppConfig)` 的稳定 JSON（`$defs` 键序归一、两空格缩进、末尾换行）到 `configs/config.schema.json`；路径以 `CARGO_MANIFEST_DIR` 上溯两级定位仓库根。
  - 测：同进程两次渲染字节相同（幂等）。

- [ ] 2. **漂移门禁**（config 插件）：单测比对渲染结果与仓库产物；不一致时提示 `just gen-config-schema`。随既有 `just test` 进 `just qa`，不新增脚本门禁。
  - 测：人为改产物可复现红（本地核对一次，不入库）。

- [ ] 3. **产物刷新**：跑一次生成，修正当前漂移（补 `tool_batch` / `tui` / `otel` / `loop_example` / `session.max_turns`）。

- [ ] 4. **维护入口**：`justfile` 增 `gen-config-schema`（不进 qa）；`configs/example.yaml` 顶部注释指向该入口。

- [ ] 5. **BDD 落地**（`runtime-config`）：新增规则「schema 产物同源」+ 两个可执行场景（产物与代码一致、覆盖当前顶层节）；`tests/bdd` 绑定 + 步骤。
  - 测：`cargo test --all-features --test bdd` 绿。

- [ ] 6. **门禁**：`just qa` 全绿；`llman sdd validate c2949-fix-config-schema-artifact --strict` 绿。
