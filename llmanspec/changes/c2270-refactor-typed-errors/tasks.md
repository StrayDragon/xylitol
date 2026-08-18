# Tasks: c2270-refactor-typed-errors

大范围机械重构：按切片硬切，禁止 `Result<_, String>` 与 typed 双路径并存。

## 1. Branch binding 与 Specs landing

- [x] 1.1 `llman sdd change start c2270-refactor-typed-errors`（或已在 feature 分支则 `attach`）
- [x] 1.2 [blocked-by: 1.1] 按 design §6 在 `llmanspec/specs/protocol-app/spec.toon` 增加 `pa-e2`（稳定 kind、禁止子串猜 session/export/trust 分类、message 不叠前缀）；`scenarios[]` 仅 `feature: false` 文档行。不改 atb14。
- [x] 1.3 [blocked-by: 1.2] `llman sdd validate c2270-refactor-typed-errors --strict --no-interactive --no-check`；commit Specs landing；`llman sdd show … --json` 确认 `readyToImplement=true`

## 2. 库错误类型与 infallible factory

- [x] 2.1 [blocked-by: 1.3] 在 `protocol::error` 增加 `XyStoreError` / `XyExportError` / `XyTrustError`（thiserror + `kind()`）；`XyError::Session` 改为包 `XyStoreError`；`lib.rs` 精选 `pub use`；单测 Display/`kind`/`From`
- [x] 2.2 [blocked-by: 2.1] `XyModelBuilder`、`build_provider` / `build_adapter`、`AgentBuilder::build` 去掉 `Result`；删 `BootstrapError::BuildFailed`；更新全部闭包与 `ModelManager::build_current_model`
- [x] 2.3 [blocked-by: 2.2] 编译过：`cargo test -q --lib -- protocol::error` 及 builder 相关测

## 3. Session store 切片

- [x] 3.1 [blocked-by: 2.1] `XySessionStore` 全部 `Result<_, String>` → `XyStoreError`；`parse_session_jsonl` / `enforce_session_version` / tree 失败臂对齐
- [x] 3.2 [blocked-by: 3.1] `SessionManager` 及 `impl XySessionStore` 跟签名；`EmptyLeafStore`、BDD fixtures、debug seed 同步
- [x] 3.3 [blocked-by: 3.2] 跟随：compaction orchestrator（Store vs 产品文案分臂）、`session_ops` / `compact_ops` / `stats` / react persist、`session_export` 不再把 store 当 opaque String
- [x] 3.4 [blocked-by: 3.3] `From<XyStoreError> for XyDriverError`：NotFound 单层 Display；Driver 去掉这些路径上的 `map_str` / `from_opaque`

## 4. Export / Trust

- [x] 4.1 [blocked-by: 2.1] `XyExportIo` + `StdExportIo` + MockExportIo → `XyExportError`；`session_export` 映射到 `XyDriverError::Io`，禁止 `XyError::Session`
- [x] 4.2 [blocked-by: 2.1] `XyTrustStore` + `TrustManager` → `XyTrustError`；Driver persist 路径 `From` 到 `Io`

## 5. 其余生产 String 错误

- [x] 5.1 [blocked-by: 2.1] config：`AppConfig`/`McpServerConfig` validate、template、value 解析改用 `LoadError` 或域 Error；文案不变
- [x] 5.2 [blocked-by: 2.1] MCP client 六个 `Result<_, String>` → 域 Error
- [x] 5.3 [blocked-by: 2.1] TUI：keybindings / themes / terminal_guard / external_editor / host override；上到 Driver 时 kind 为 InvalidInput 或 Io
- [x] 5.4 [blocked-by: 2.1] clipboard / image / browser / settings / mutation / otel 装配
- [x] 5.5 [blocked-by: 2.1] `packages/xylitol-ai-bridge` tokenize 下载/删缓存；lab/example 不强制

## 6. 收口 from_opaque 与闸

- [x] 6.1 [blocked-by: 3.4, 4.2, 5.3] 生产路径不再对 session/export/trust/config/MCP 走 `from_opaque`；更新 `driver_error` 单测；残留 opaque 才回落 Message
- [x] 6.2 [blocked-by: 6.1] `rg 'Result<[^,]+,\s*String>' src/ packages/xylitol-ai-bridge/src`：生产文件零命中（允许 test/lab/example）
- [ ] 6.3 [blocked-by: 6.2] `just fmt` + `just lint` + `just qa`
- [ ] 6.4 [blocked-by: 6.3] `llman-sdd-verify`；全绿后 `llman sdd change finalize c2270-refactor-typed-errors`
