# Tasks — c610-add-driver-session-tree-kind

- [x] 1.1 增加 `SessionTreeKind` + `SessionTreeTravel`（domain 或 `app/core`；不加 Xy）
- [x] 1.2 `Driver` trait：`session_tree` / `travel_session_tree`；InProcess 实现 MessageHistory（含 user travel 语义）
- [x] 1.3 未实现 kind → 明确 `Err`；单测覆盖 user / non-user / unsupported
- [x] 2.1 REST 读树 + travel；RemoteDriver 对接
- [x] 2.2 ScriptedDriver / harness stub 可编译并记录或返回桩
- [x] 3.1 若引入 `protocol::Command`，经 dispatch；否则文档写明仅 Driver 方法
- [x] 4.1 `llman sdd validate c610-add-driver-session-tree-kind --strict --no-interactive`
- [x] 4.2 `cargo test -p xylitol --lib`（至少覆盖新 Driver/session 测）与相关 server 测
