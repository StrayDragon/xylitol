# Tasks — c996-update-agent-hooks-model-select

- [ ] 1. `llman sdd validate c996-update-agent-hooks-model-select --no-interactive`
- [ ] 2. `select_model` / `cycle_model` 成功后 dispatch `model_select`
- [ ] 3. `set_thinking_level` 后 dispatch `thinking_level_select`
- [ ] 4. 扩展 wiring 操作字典 + feature 例子；录制断言
- [ ] 5. `cargo test --test bdd -- --test-threads=1`；`validate --strict`
