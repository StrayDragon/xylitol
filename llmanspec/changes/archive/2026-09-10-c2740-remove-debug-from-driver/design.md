# Design: debug 退出 Driver；Host 泵

> Designed / pre-start。依赖 `c2710`。

## 1. Debug 退出产品信封

事实：

- `XyDriver::load_debug_scene`（proto + in_process/session.rs + remote + harness）
- `src/app/tui/effects/slash/debug.rs` 调 Driver
- `src/app/debug_fixtures/mod.rs` 写明 Driver + slash 为入口
- `host.rs` 对 debug method 转发
- harness 部分测 **禁止** 调 `load_debug_scene`（list/verify-smoke）——产品路径本就矛盾

落地：

1. Command / registry / host 删除 debug。
2. TUI：仅 `cfg(debug_assertions)` 或测试 harness 的 `inject_scene`，写内存 session，不走 RPC。
3. 远程 Driver 对未知 debug 不再实现（方法删除后自然编译失败）。

## 2. Host 泵（次段）

`just complexity` 上 TUI resume `handle_input`、`poll_mcp_bootstrap` 等偏高。硬闸仍是 host/layout/effects/bridge **入口**。本段只：把 debug 与会话 RPC 解耦后，将 Host 内「本该在 effects」的分支搬回 effects。禁止借机拆主 crate。

若次段超出 ~1 PR，认领人在 tasks 勾选「Host 另 PR」并在 Further Notes 说明，不要阻塞 debug 删除合入。

## 3. 验证

- `rg load_debug_scene` 仅 fixture/harness。
- TUI 复杂度入口不升。
- `just test-tui` 与 `just qa`。
