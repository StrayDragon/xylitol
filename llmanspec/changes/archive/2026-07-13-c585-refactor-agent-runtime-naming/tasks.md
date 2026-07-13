# Tasks — c585-refactor-agent-runtime-naming

- [x] 1. Rename `ReActAgent` → `AgentRuntime`；`session::Agent` → `AgentCapabilities`；更新 re-export / builder / app/core / embed
- [x] 2. 更新 specs 点名语句（本 change deltas）与 `agent/mod` 文档
- [x] 3. 更新 tests/bdd、API snapshot、write-surface skill
- [x] 4. `llman sdd validate c585-… --strict --stage spec` + `cargo test -p xylitol --lib` 相关绿 + `just qa`
