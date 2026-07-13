# Tasks — c665-update-app-tui-abort-feedback

- [x] 1. Delta specs 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c665-update-app-tui-abort-feedback --strict --no-interactive`
- [x] 2. `Driver::execute_bash` → `&self`；更新 InProcess / Remote / Scripted / dispatch mocks
- [x] 3. Host：`note_user_abort` + bang `bash_active` / status Running；effects 去掉 bash await
- [x] 4. `run_host_loop`：`select!` 并发 bang 与输入；Esc 调 `abort`
- [x] 5. Bridge：`Aborted` 文案与去重
- [x] 6. Harness：abort 反馈 + bang Esc；`just fmt` + 相关测绿
