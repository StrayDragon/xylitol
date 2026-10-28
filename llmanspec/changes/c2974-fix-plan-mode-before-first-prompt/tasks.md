# Tasks

- [ ] 1. **模式切换**（kernel）：`set_plan_mode` 将 store `NotFound` 视为空分支，其他错误照常上抛；首次 `/plan` 直接进入 plan 模式。测：绑定未持久化 session 后 toggle 返回 plan，再 toggle 返回 execute。
- [ ] 2. **回归测试**（kernel）：在 bootstrap driver harness 中覆盖 pending store 路径，并断言 plan_mode entry 已写入分支。测：`cargo test -p xylitol-kernel plan_mode_before_first_prompt`。
- [ ] 3. **门禁**（repo）：`just qa` 全绿；`llman-sdd validate --all --strict` 通过。测：`just qa` + validate。
