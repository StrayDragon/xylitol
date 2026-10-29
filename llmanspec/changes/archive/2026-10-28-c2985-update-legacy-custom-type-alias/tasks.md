# Tasks

- [x] 1. **specs**（spec）：`agent-session-store` 的 `r2941` / `r2945` 明确会话加载时把旧 `plan_checkpoint` / `plan` 一次性迁移为新名（只追加、幂等），读取优先新名、旧名仅作迁移输入。测：`llman-sdd validate --all --strict`。
- [x] 2. **sdk**（session）：`XySessionStore` 增加默认 `migrate_legacy_custom_types`（读叶分支、按 `(新名, data)` 去重、追加新名条目）；新增旧名到新名的映射 helper。测：`cargo test -p xylitol-plugin-sdk legacy_custom`。
- [x] 3. **runtime**（session）：`switch_session` 与 `load_conversation_history_from_store` 在加载时调用迁移，失败只 warn 不中断。测：`cargo test -p xylitol-runtime`。
- [x] 4. **tests**（sdk）：旧 `plan` / `plan_checkpoint` 迁移为新名、只追加、重复加载不重复追加；已存在新名条目时不再迁移。测：`cargo test -p xylitol-plugin-sdk`。
- [x] 5. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
