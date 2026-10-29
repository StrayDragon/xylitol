# Tasks

- [x] 1. **specs**（app-tui-fixed-zone）：扩展 `r2936`，写明已加载缓冲上限、超限淘汰最旧条目并回退为“加载更早”，并补一条可执行场景。测：`llman-sdd validate c2991-bound-loaded-history-buffer`。
- [x] 2. **app-tui**（history_window）：新增 `HISTORY_BUFFER` 与 `enforce_buffer_cap`；`new` / `merge_older` 超限淘汰并置 `has_more=true`，重算 `first_entry_id`。测：`cargo test -p xylitol-app-tui history_window`。
- [x] 3. **app-tui**（host）：`prepend_history` 与 `apply_switched_session` 使用缓冲真实 `has_more`。测：`cargo test -p xylitol-app-tui`。
- [x] 4. **tests**（repo）：补缓冲上限单测与 BDD 场景步骤/绑定。测：`cargo test --all-features --test bdd`。
- [x] 5. **门禁**（repo）：`llman-sdd validate --strict`、`just qa` 全绿。测：`just qa`。
