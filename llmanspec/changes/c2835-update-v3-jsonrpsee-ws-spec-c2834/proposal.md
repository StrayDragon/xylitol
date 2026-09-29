---
depends_on: []
---

# 线协议 v3 硬切收尾(c2834 后继)

## Why

c2834 已落地 v3 双轨全链路(真源/codegen/服务端/客户端/对拍全绿/TUI 产品面
v3),但 JSON-RPC 的 **jsonrpsee 分发依赖与 WS 文本帧产品路径仍在服役**。
本 change 完成硬切收尾:退役 jsonrpsee、WS 文本帧降为纯调试或移除、spec
旧条款改写为 v3 终态、`PROTOCOL_VERSION` bump。

## What Changes

- `handle_uplink` 内核替换:`dispatch_raw`(jsonrpsee)→ `HostState::handle_unary`
  直调(InProcess 同源);**须审计并保住** jsonrpsee 层承载的幂等准入
  (r1781 CALL scope)、审批/问卷分发(r1771/r1772)与 -32601 语义。
- WS `/rpc` 文本帧产品路径退役(binary 唯一产品上行;text 处置:断连或
  显式调试模式,按 spec 收口措辞定)。
- POST `/rpc` JSON 形态定位为调试通道(r1784 调试语义),实现去 jsonrpsee
  化(codec 手工信封 + handle_unary 直调)后移除 jsonrpsee 依赖。
- spec 终态改写:server-core r1778/r1796/r1803/r1804/r1809、protocol-app
  r1701/r1709/r1696 等 JSON-RPC 钉死措辞(r1902/r1909 的迁移期条款同步
  收口);四象限 envelope 旧类型清理(ServerHello 等死变体)。
- `PROTOCOL_VERSION` bump(2→3)与 attach 预检联动。
- 租约跨连接窗口专项(多 client re-mint 竞态,c2834 tasks 5.1 备注)。

## Capabilities

- `server-core`(信封终态 + 幂等/审批语义迁移审计)
- `protocol-app`(JSON-RPC 措辞 → v3 终态)

## Impact

- 代码:`src/app/server/rpc_module.rs`(内核替换)、`http.rs`(text 帧
  处置)、`wire/envelope.rs`(旧类型清理);依赖:− jsonrpsee。
- 测试:steps_server 的 JSON POST 场景改走调试通道语义或 v3(逐场景审计)。
- 参考:c2834 全套 research/ 与 design §8(迁移纪律 r1909)。
