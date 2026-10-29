# Tasks

- [ ] 1. **sdk**（session）：新增 `has_legacy_custom_types(&[SessionEntry]) -> bool`，覆盖 `plan` / `plan_checkpoint`，并从 `session` 模块导出。测：`cargo test -p xylitol-plugin-sdk has_legacy_custom_types`。
- [ ] 2. **runtime**（session）：`load_conversation_history_from_store` 先读一次分支；仅检测到 legacy 时迁移并重读。测：`cargo test -p xylitol-runtime session`。
- [ ] 3. **tests**（runtime）：用计数 store 断言非 legacy 会话每 run 只读一次；legacy 会话仍迁移且幂等。测：`cargo test -p xylitol-runtime`。
- [ ] 4. **门禁**（repo）：`llman-sdd validate --strict`、`just qa` 全绿。测：`just qa`。
