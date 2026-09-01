---
depends_on: []
---

# Staged Wire Import：import_jsonl 的 content 载荷暂存导入

## Why

`/session-import` 是有 MUST 合约的用户能力（app-tui-commands：确认后 MUST ImportJsonl 并 switch/重建 transcript），而产品 TUI 默认拓扑是 attach → RemoteDriver。该路径下客户端读取本地文件后发送 `{"content": …}`，服务端却要求 `input_path`（或遗留别名 `path`）——**默认拓扑必报 `missing input_path`，MUST 合约在产品主路径未兑现**。

export 侧已有对称先例（staged export：无 output_path 时 Host 暂存临时文件、dispatch 后读回内容并清理）。import 缺的正是对称的暂存块。BDD 全绿系因 wire 导入链路无可执行场景覆盖——本 change 同时补上场景，防回归。

## What Changes

- `dispatch_session_unary` writer 路径为 `import_jsonl` 增加内容暂存：载荷无 `input_path`/`path` 且携带 `content` 时，Host 将内容写入唯一临时输入路径（`xylitol-import-*.jsonl`），dispatch 后清理；携带 `input_path`/`path` 时行为不变；
- 响应无需改动：dispatch 的 `DispatchOutcome::NewSession(id)` 已映射为 `{"session_id": id}`，与 RemoteDriver 既有期待一致；
- `server-core.feature` 落 `sr-imp1` 规则 + 可执行场景（content 载荷 wire 导入兑现、暂存文件不残留）。

## 非目标

- 客户端零改动（`{"content"}` 载荷本就是正确形状）；
- 不做跨机分块传输等远程优化（content 全量推送已满足本机与 SSH 隧道拓扑）；
- `Event::Response` 死变体等相邻死码不在本票（留 dead-code audit）。

## Impact

- `src/app/server/host.rs` writer 分支新增暂存块（紧邻 staged_export，~15 行）；
- `llmanspec/specs/server-core/server-core.feature` +1 规则 +1 可执行场景；`tests/bdd/steps_server.rs` / `bindings_server.rs` +1 组步骤与绑定；
- 门禁基线：既有 BDD 432 场景全绿基础上 +1。
