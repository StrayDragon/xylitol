# Tasks — c462-demo-tool-status-bg

- [x] 1. Delta specs：`app-tui-transcript` 增补工具块三态全行 bg requirements
- [x] 2. demo：`ToolBlockStatus` + truecolor bg 闭包（对齐 DESIGN.md RGB）
- [x] 3. Tool / Diff 渲染：块内行 `apply_background_to_line`；seed/script 设状态（含 pending→success）
- [x] 4. harness：断言 success/error/pending 的 cell bg（VirtualTerminal `Color::Rgb`）
- [x] 5. 回写 `design/expandable.md`（demo 已验证）
- [x] 6. `cargo test -p xylitol-tui --test agent_demo_test`；`llman sdd validate c462-demo-tool-status-bg --strict --no-interactive`
