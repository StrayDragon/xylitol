# design — c530 公开嵌入 API

## 目标形状

```text
外部 crate / 自建 client
  → xylitol::embed::{bootstrap, InProcessDriver, …}
  → Driver::run → XyEvent 流
  → 自有渲染
```

同时继续可用：`xylitol::{XyModel, XyEvent, …}` 精选契约。

## 导出策略（推荐）

| 符号 | 导出 |
|---|---|
| `BootstrapInput` / `bootstrap` / `BootstrappedAgent` | embed |
| `BuildAgentOptions` / `build_agent` | embed |
| `Driver` / `InProcessDriver` / `EventStream` | embed |
| `McpSession` | embed |
| `dispatch` | embed（可选；TUI/Server 接线后） |
| `infra::*` 具体类型 | **不**进 embed |
| `agent::session::*` | **不**进 embed |

**定稿（task 1）：** 采用选项 1 — crate 根 `pub mod embed` re-export `app::core` 子集；`app::core` 保持 `pub(crate)`（外部只经 `xylitol::embed::*`，不经 `app::core` 路径）。不加 `feature = "embed"`（与精选 `Xy*` 同默认可用）。

## 非目标

- 保证 1.0 前 semver 冻结全部 `pub mod`
- RemoteDriver 完整（属 c540）
