# Tasks — c660-fix-agent-tool-abort-process

- [x] 1. Delta specs 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c660-fix-agent-tool-abort-process --strict --no-interactive`
- [x] 2. `BashExecHandler`：cancel 用 `Mutex`；`abort`/`abort_bash` 支持 `&self`
- [x] 3. `AgentRuntime::abort` 调用 `inner.abort_bash()`（保留清 steer / 不粘滞 token）
- [x] 4. 合成测：交互 bash `sleep` + `abort` → `cancelled`；相关既有测绿
- [x] 5. `just fmt` + 相关 `cargo test`（agent session / runtime / bash_exec）通过
