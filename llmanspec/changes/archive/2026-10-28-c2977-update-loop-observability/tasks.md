# Tasks

- [x] 1. **specs**（spec）：`agent-runtime` r2924 语义属性；`infra-otel` session 覆盖加入 `loop.middleware`。测：`llman-sdd validate --all --strict`。
- [x] 2. **SDK**（crate）：`LoopStep.properties` + helpers。测：`cargo test -p xylitol-plugin-sdk`。
- [x] 3. **runtime**（crate）：`run_stage` / `call` 接 `ObsSessionContext` 与 Langfuse 属性。测：runtime 单测。
- [x] 4. **plugins**（plugins）：plan / workflow / todo / loop-policies / MCP 填语义属性。测：各插件单测。
- [x] 5. **docs**（repo）：README 属性约定。测：`cargo doc`。
- [x] 6. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
