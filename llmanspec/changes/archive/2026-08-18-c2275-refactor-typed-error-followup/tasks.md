# Tasks: c2275-refactor-typed-error-followup

无 live specs。`skip_specs_landing: true`。

## 1. MCP 文档

- [x] 1.1 `connect_and_discover*` rustdoc：空列表 `None`；失败走 diagnostics

## 2. 会话域拆分

- [x] 2.1 `XySessionError` + `XyError::Session` 改包；`XyStoreError` 去掉 NoActiveSession
- [x] 2.2 session_ops / compact_ops / react busy / tree travel 改走 session 域
- [x] 2.3 `CompactionError::Session`；库入口 `pub use XySessionError`

## 3. Driver 观测

- [x] 3.1 叶臂 `source_kind`；flatten 写 Session/Export/Trust
- [x] 3.2 `detail_kind` / `log_failure` 测：kind=NotFound 且 detail=Session

## 4. 叶 variant

- [x] 4.1 ClipboardError / ImageError enum + 调用点
- [x] 4.2 McpError Config/Timeout；AiBridgeError::Io(`std::io::Error`)
- [x] 4.3 `src/AGENTS.md` 错误与观测段

## 5. 验证

- [x] 5.1 `cargo test --lib` 1367 passed；`cargo test --test bdd` 276 passed；`just qa` 1649 passed + live-provider 绿
