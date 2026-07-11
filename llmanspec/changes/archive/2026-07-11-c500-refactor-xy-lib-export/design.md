# design — c500 精选导出与死包装

## 导出面

`src/lib.rs` 精选 `pub use`，建议最小集：

- 端口：`XyModel`、`XyTool`、`XySessionStore`、`XyEventSink`、`XyPermission`、`XyBashExecutor`、`XyExportIo`、`XySecretResolver`、`XyModelBuilder`
- 事件/流：`XyEvent`、`XyChunk`、`XyStream`
- 错误：`XyError`、`XyToolError`（若作为库错误面）
- 配置元数据：`XyModelConfig`、`XyModelKind`、`XyModelMeta`、`XyToolSchema`

不导出：`Driver`、infra 具体类型、内部协作者。

## 删除

| 符号 | 理由 |
|---|---|
| `SessionIO` | 纯转发，无调用 |
| `PermissionGate` | Arc 壳 |
| `LlmMessageConverter` | 零 impl |
| `XyToolDefinition` | 无消费者 |

`Agent` 直接持有 `Arc<dyn XyPermission>` 与 `Arc<dyn XySessionStore>`。

## 迁移

一次性改调用点；无兼容 shim（符合 AGENTS）。
