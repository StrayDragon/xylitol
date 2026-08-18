---
depends_on: []
branch: sdd/c2270-refactor-typed-errors
base_sha: 9bf6dca3adf9893cd67a8495f72a4f2ab691e9c8
checkpointed: false
---

# 全仓收口 Result String 错误：port typed error + 稳定 kind

> **一句话**：把库契约与生产路径上的 `Result<_, String>` 换成 typed error（或去掉不会失败的 Result），Driver / `XyEvent::Error` 的 kind 在源头产生，不再靠文案子串猜测；用户可见 message 去掉叠加前缀。
> **目的地**：session / export / trust 等 port 有 `Xy*Error`；infallible factory 直接返回值；config / MCP / TUI / clipboard / tokenize 生产 API 不再用裸 `String` 当错误。

## Why

1. **Port 圈落后于热路径**。`XyError` / `XyToolError` / `XyDriverError` 已 typed，但 `XySessionStore`、`XyExportIo`、`XyTrustStore` 与 `XyModelBuilder` 仍是 `Result<_, String>`。Driver 用 `from_opaque` 对 Display 做子串分类，kind 不稳定，且 `NotFound` 的 Display 会叠成 `not found: session not found: …`。
2. **假 Result 制造包装税**。`AgentBuilder::build`、`build_provider` / `build_adapter` 实际 `Ok(...)`；桥包 `build_adapter_with_wire_policy` 已是 infallible。主仓闭包却把 `XyModelBuilder` 钉成 `Result<Arc<dyn XyModel>, String>`，`ModelManager` 再 `anyhow!(e)` 进 `XyError::Provider`。
3. **类型说谎**。`session_export` 把 export IO 失败 `map` 成 `XyError::Session`。compaction / parse 跟 store 共用 `String`，产品文案与 IO 无法区分。
4. **Pre-0.0.1**：无外部 SemVer 客户，禁止双解析路径；本票一次性改调用点，不留 `From<String>` 假类型化。

## What Changes

1. **库契约**
   - 新增 `XyStoreError` / `XyExportError` / `XyTrustError`（thiserror + 稳定 `kind()`），挂在 protocol 并精选 `pub use`。
   - `XySessionStore` / `XyExportIo` / `XyTrustStore` 签名改用上述类型；`dyn` 口禁止 associated `Error`。
   - `XyError::Session` 从 `anyhow::Error` 改为包 `XyStoreError`。
   - `XyModelBuilder` → `Arc<dyn Fn(&XyModelConfig) -> Arc<dyn XyModel> + Send + Sync>`；`AgentBuilder::build` → `AgentRuntime`；`build_provider` / `build_adapter` 去掉 `Result`。
2. **直连实现与跟随缝**：`SessionManager`、`StdExportIo`、`TrustManager`、compaction orchestrator、session parse/tree、`session_export`、Driver `map_str` 全部跟签名走；export 失败进 Driver `Io`，不再冒充 Session。
3. **其余生产 `Result<_, String>`**（config 校验、MCP client、TUI chrome、clipboard/image/browser、settings、mutation、otel 装配、bridge tokenize）：各域 crate 私有（或已有）error enum，**禁止**再引入 `Xy*` 品牌；lab / example / 纯测 helper 不强制。
4. **Display / kind**：session/export/trust/config/MCP 失败的 kind 在源头产生；`from_opaque` 不再作为这些路径的分类器（可留作真正 opaque 残留的 Message 回落）。用户可见 message 去掉叠加前缀。
5. **Specs**：`protocol-app` 增补产品级 kind 条款（不写类型名/路径）；验证靠现有 BDD + 单测 + `just qa`，不新开 `.feature`。

## 非目标

- 为未发布 API 保留 `Result<_, String>` 别名或 `XyStoreError::Message(String)` 万能臂
- 把 `XyError::Config(String)` / `XyError::Provider(anyhow)` 再拆一票（可顺手，但不作为本票完成判据）
- lab / example / `live_tape::check_plain` 等测专用 `Result<_, String>`
- 改变 compaction 产品文案语义（empty / disabled 原句保留，只换载体）
- 插件式 associated `Error` on `dyn` port

## Capabilities

- `protocol-app`：增补「失败 kind 在源头、禁止子串猜分类、message 不叠前缀」（落地文本见 design）
- 复核：`app-tui-bridge` atb14（消费 kind，不改陈述）；`agent-session-store` / `domain-compaction` / `infra-mcp` / `runtime-config` 不钉类型名

## Impact

- 库公开签名破坏性（Pre-0.0.1 允许）：`XySessionStore`、`XyExportIo`、`XyTrustStore`、`XyModelBuilder`、`AgentBuilder::build`、`XyError::Session`
- 嵌入方 / 测试 stub 的 `impl XySessionStore` 必须改错误类型
- 用户可见错误串：Driver 叠加前缀消失（如 `not found: session not found: X` → 单层 `not found: …` 或 store 原句择一，见 design）；`XyEventError.kind` 对上述域不再无故落 `Message`
- `BootstrapError::BuildFailed` 随 `build()` 不再失败而删除或收窄

## Test seams

复用现有 harness，不新开 `.feature`：

- `XyError` / `XyDriverError` / 新 `Xy*Error` 单测：`kind()`、`From`、Display 无叠加前缀
- 现有 session / compaction / trust / export BDD：断言更新到新 Display；kind 不再依赖 `from_opaque` 子串
- `protocol-app` pa-e1 既有 round-trip；新条款用 `feature: false` 文档场景 + 单测
- 闸：`just qa`（含 lint / 主测 / TUI 测）
