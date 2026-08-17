# Design：typed port errors 与稳定 kind

## 1. 现状

热路径已有 `XyError` / `XyToolError`；整机缝已有 `XyDriverError`（含 `from_opaque`）。仍用 `String` 的库口：

| 契约 | 现状 | 实际失败模式 |
|---|---|---|
| `XySessionStore` | 11 个 `Result<_, String>` | NotFound / Io / Serialize / Validation / Unsupported |
| `XyExportIo` | 读写 `String` | 创建目录 / 读 / 写 Io |
| `XyTrustStore` | `set_trust -> Result<(), String>` | lock / 读 / 写 / JSON |
| `XyModelBuilder` | `Result<Arc<dyn XyModel>, String>` | **不会失败**（桥包 factory 已 infallible） |
| `AgentBuilder::build` | `Result<AgentRuntime, String>` | `Ok(materialize_runtime())` |

`SessionManager` 上另有 ~40 个同形签名，是 port 的具体实现，必须一起改，否则立刻 `to_string()`。

## 2. 错误类型（库入口才 `Xy*`）

`dyn Trait` 不能 associated `Error`。具体 enum，禁止万能 `Message(String)`。

```text
XyStoreError
  NotFound { session_id }
  EntryNotFound { entry_id }
  Io { op: &'static str, #[source] io::Error }
  Serialize(serde_json::Error)
  Validation { message }   // 版本不符、CWD、坏 JSONL 拒绝
  Unsupported { op }       // 默认 delete_session 等

XyExportError
  Io { path, #[source] io::Error }

XyTrustError
  Lock(io::Error)
  Io(io::Error)
  Parse(serde_json::Error)
```

`kind()` 用 `IntoStaticStr` 与现有 `XyError` 同形。

映射：

```text
XyStoreError ──► XyError::Session
                 └── Driver flatten：NotFound / Io / Unsupported / Message
                     （禁止再包一层 Agent，否则 kind 变成 Agent、Display 叠前缀）
XyExportError ─► XyDriverError::Io     （禁止再进 Session）
XyTrustError ──► XyDriverError::Io
配置 / MCP / TUI 私有 Error ─► XyDriverError 对应臂（InvalidInput / Io / Message）
```

## 3. Display 清理（已拍板）

今日：`from_opaque("session not found: abc")` → `NotFound("session not found: abc")` → Display `not found: session not found: abc`。

本票：源头 kind + **单层** Display。

- Driver `NotFound` 仍是 `not found: {body}`，`body` 只含资源身份或 store 原句中去掉已含 “not found” 的重复，不把完整 `err.to_string()` 再塞进已有前缀的臂。
- 实现优先：`From<XyStoreError>` 对 `NotFound` 映射为 `XyDriverError::not_found(session_id)`（或等价短 body），**不要** `from_opaque(err.to_string())`。
- compaction 产品句（empty session / compaction disabled）保持原文，kind 用 `Message` 或独立臂，不加 `not found:` / `io:`。
- `XyEventError.kind` 对齐外层 `XyError::kind`（Session/Config/…）—— atb14 分流不变。

## 4. 不会失败的 factory

直接返回：

- `build_adapter` / `build_provider` / `build_provider_with_hooks`
- `XyModelBuilder`
- `AgentBuilder::build`

测试闭包从 `Ok(Arc::new(...))` 改为 `Arc::new(...)`。`BootstrapError::BuildFailed` **保留**（装配期 `create_dir_all` 等 IO 仍会失败；factory 本身不再 `Result`）。`ModelManager::build_current_model` 不再 `map_err` 到 `Provider`。

Pre-0.0.1：**禁止**新旧签名并存。

## 5. 全仓其余 String 错误（同票，按域私有类型）

不升 `Xy*`。已有类型则复用（`LoadError`、`TodoValidationError`、`RuntimeControlError`）。

| 域 | 方向 |
|---|---|
| config validate / template | `LoadError::Validation` 或 `ConfigValidateError`；文案不变 |
| MCP client | `McpError { Connect, Call, … }` |
| TUI keybindings / themes / terminal_guard / external_editor | 面私有 Error → 上到 Driver 时 `InvalidInput` / `Io` |
| clipboard / image / browser / settings / mutation | `io::Error` 或域 Error |
| bridge tokenize 下载/删缓存 | `AiBridgeError` 扩 Io 臂或 tokenize 模块私有 Error |
| otel 装配 | 已有 `Result<_, String>` → 并入装配 Error |

lab / example / 测 helper 保持 `String` 可以。

## 6. Specs（产品级，不写类型名）

`protocol-app` 新增条款（req_id 实现时取空号，建议 `pa-e2`）：

> 应用缝与 `XyEvent::Error` 对会话持久化、导出/导入、trust 持久化失败 MUST 给出稳定 kind（至少能区分缺失 / IO / 校验 / 不支持），MUST NOT 仅凭错误文案子串猜测这些域的分类。用户可见 message MUST 不把同一语义前缀叠两次。真正无结构的提示 MAY 使用 Message kind。

atb14 不改（消费侧）。本条场景 `feature: false`；可执行覆盖走单测 + 现有 BDD 断言更新。

## 7. 实施顺序（机械重构例外）

禁止「String 与 typed 双路径」。按切片硬切：类型落地 → factory → store → export/trust → 跟随缝 → 其余域 → 删 `map_str` / 收缩 `from_opaque` → `just qa`。
