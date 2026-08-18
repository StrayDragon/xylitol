---
depends_on:
- c2270-refactor-typed-errors
skip_specs_landing: true
branch: sdd/c2275-refactor-typed-error-followup
base_sha: 6f65930bde1f5151b33ed1d1d996aa9036ab60e5
checkpointed: false
---

# c2270 后续：会话域拆分、Driver 来源观测、叶错误 variant

> **一句话**：把控制面失败从 `XyStoreError` 挪到 `XySessionError`；Driver flatten 后 `kind` 不变、`detail_kind`/`source.kind` 能看出 Session/Export/Trust；clipboard/image/MCP/bridge Io 用真正的 variant，不再只包一层 String。
> **目的地**：持久化 port 仍只报 store 失败；无绑定会话 / busy / 树旅行走 session 域；MCP 装配失败只走 diagnostics（rustdoc 写清）。

## Why

c2270 把 `Result<_, String>` 收成 typed error 之后，仍有三处类型说谎：

1. `XyStoreError` 同时表示 JSONL 持久化失败和「没有绑定会话 / busy / 内存树里找不到 entry」。
2. Driver 把 `XyError::Session` flatten 成 `NotFound`/`Io` 之后，日志只剩 Driver 分类，看不出错误来自 session 域。
3. clipboard / image / `AiBridgeError::Io` / MCP connect-timeout 仍是 String 外套；`TuiSurfaceError` 已有 InvalidInput/Io 分叉，只需保持。

## What Changes

1. 新增库入口 `XySessionError`（Store / NoActiveSession / Busy / EntryNotFound）；`XyError::Session` 改包它。`XyStoreError` 去掉 `NoActiveSession`。`XySessionStore` 签名仍是 `XyStoreError`。
2. `XyDriverError` 叶臂带可选 `source_kind`：flatten 后 `kind()` 仍是 `NotFound`/`Io`/…；`detail_kind()` 与 `log_failure` 的 `source.kind` 为 `Session`/`Export`/`Trust`。
3. `ClipboardError` / `ImageError` 改为 Io/Unsupported/Decode（image 另有 Empty/Limit）；`McpError` 增 Config/Timeout；`AiBridgeError::Io` 改为 `std::io::Error`。Driver 对 clipboard Unsupported/Decode 分别映到 Unsupported/InvalidInput（Display 前缀会变，这是有意的 kind 对齐）。
4. `connect_and_discover*` rustdoc：空列表才 `None`；连接失败走 `diagnostics`，不走返回值。

## 非目标

- 改 `pa-e2` / atb14 / 用户可见 persist message（`not found: {id}` 单层）
- 把 `XyError::Config(String)` / `Provider(anyhow)` 再拆
- 为 clipboard/image 引入 `Xy*` 品牌
- 恢复 `auto-compaction:` Display 前缀（产品句在 CompactionEnd.reason / policy 原文）

## Capabilities

- `protocol-app`（无新 req；观测字段是 Driver 日志，不是产品 MUST）
- `infra-mcp`（文档合同，已有 diagnostics 行为）
