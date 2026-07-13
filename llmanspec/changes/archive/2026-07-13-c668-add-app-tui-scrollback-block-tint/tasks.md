# Tasks — c668-add-app-tui-scrollback-block-tint

- [x] 1. Delta 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c668-add-app-tui-scrollback-block-tint --strict --no-interactive`
- [x] 2. `UiEntry::Bash` + bridge/host bang 上行（begin → pending；完成/取消更新 status/output）
- [x] 3. `scrollback.rs`：bang 块 tint + 块间一行 Spacer；user/tool 对齐 demo（无块内 padding_y）
- [x] 4. 修订 `design/bash-mode.md`（块 tint MUST）
- [x] 5. Harness：pending/success/cancelled tint 或 CSI 痕迹；`(cancelled)` vs `Aborted`；块间隙
- [x] 6. `just fmt` + 相关 `cargo test` / clippy 绿
