# Tasks — c600-update-app-tui-session-tree-travel

- [x] 1.1 demo：`travel_to_history` 按 kind=user / 非 user 分支（leaf、transcript、editor）
- [x] 1.2 移除或降级 `travel_path_with_replies` 作为 Enter 默认路径
- [x] 2.1 更新/替换 harness：废除 keeps-reply；加 user 预填 + leaf=父；非 user 不预填
- [x] 3.1 `session-tree.md` / `session-tree-vs-pi.md` / `keybindings.md` 对齐 pi travel
- [x] 3.2 playground：Enter user → 填 input 注脚 + 与 session-tree MUST 同形
- [x] 4.1 `llman sdd validate c600-update-app-tui-session-tree-travel --strict --no-interactive`
- [x] 4.2 `cargo test -p xylitol-tui --test agent_demo_test`（travel/fork 相关）
