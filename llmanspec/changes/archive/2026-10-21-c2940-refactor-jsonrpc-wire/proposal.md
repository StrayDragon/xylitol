---
depends_on: []
title: 产品线协议一步到位迁移为 JSON-RPC 2.0 文本信封（salvo 载体）
needs_specs_change: true
branch: sdd/c2940-refactor-jsonrpc-wire
base_branch: main
base_sha: 0da3e7c6d1c54976be465bcfc08ff7d500a82ed7
---

# 产品线协议一步到位迁移为 JSON-RPC 2.0 文本信封（salvo 载体）

## Why

产品线协议当前是 fory-v3 二进制信封（四象限 `RpcMessage`，`type` 标签 + 数字 `method_id`，`.fbs` 生成代码）。问题：

- 二进制载荷不可用 curl / 普通 HTTP 工具排障，也无法被外部客户端直接消费。
- 同一语义存在两份映射（`serve` 与 `app-tui` 各一份 `XyEvent ↔ Event` 孪生），并靠 `XYLITOL_ENVELOPE_V3` 与 `application/x-fory-v3` 双轨过渡，维护成本高。
- 版本化命名（`v3`、`fory`、`WIRE_FORMAT_FORY_V3`）把载体细节扩散到类型、模块、环境变量与门禁，与「不保留双轨、一步到位」的要求冲突。

本次一步到位：产品 `/rpc` 的 HTTP body 与 WS 帧统一改为 JSON-RPC 2.0 文本，载体继续使用 salvo，不引入 jsonrpsee / axum / tracing。所有端（serve、app-tui、kernel、root、app-print）同批切换，不保留旧路径。

## What Changes

- **协议**：`xylitol-protocol` 以 JSON-RPC 2.0 信封（request / notification / response / batch）为唯一线形状；删除 `v3/` 生成物与 `envelope.fbs`；`Command` / `Event` 作为 `params` / `result` / 下行通知载荷保留。
- **新 crate**：`crates/xylitol-wire` 承载唯一的 `XyEvent ↔ protocol::Event` 映射（覆盖全部 23 个 `XyEvent` 变体，不静默丢弃）；`serve` 与 `app-tui` 都依赖它；删除四份孪生映射。
- **载体**：`xylitol-serve` 的 `POST /rpc` 接受 JSON-RPC 文本（4 MiB 上限、2048 层深度守卫），WS `/rpc` 只承载 text 帧；binary 帧拒绝并关闭；`GET /healthz` 保持现状。
- **握手**：`host.describe` 返回 `{protocol: 4, formats: ["jsonrpc"]}`；客户端校验 `protocol` 与 `formats`，不匹配即断开，不降级。
- **客户端**：`xylitol-app-tui` 的 `envelope_client.rs` 重写为 `host_client/json_rpc.rs`；`http_ws.rs` 只发 text 帧；`XyRemoteDriver::new_v3` 改名为 `new`（不留别名）。
- **命名纪律**：线协议标识符、模块、文件、常量与环境变量不再带版本后缀；删除 `XYLITOL_ENVELOPE_V3`、`WIRE_FORMAT_FORY_V3`、`application/x-fory-v3`、`xy.wire.v3`。新增 `scripts/check_no_version_names.py` 门禁（经 `just qa` 接入）。
- **构建**：删除 `fory` 依赖与 `codegen-envelope` recipe；`scripts/gen_envelope.py` 删除；`scripts/check_crate_graph.py` 增加 `xylitol-wire`，`kernel → protocol` 边合法化。
- **Specs（live）**：改写 `protocol-app`、`server-core`、`workspace-assembly`、`app-tui-bridge`、`app-tui-host`、`app-tui-transcript` 中线协议措辞；删除 `r1910`（IDL 单源生成）、`r1940`（信封开关环境变量）、`r1908`（双轨对拍）、`r1909`（迁移期保活）。`session/bash_output` 改名 `bash/output`。
- **BDD**：删除 `steps_envelope.rs`、`bindings_envelope.rs`、`dual_rail.rs`；JSON-RPC 场景绑定落在现有 per-feature 绑定文件（不新增 `bindings_jsonrpc.rs` / `steps_jsonrpc.rs`）；更新其余受影响 steps 与 occupancy 测试。
- **文档**：根 `AGENTS.md`、`src/AGENTS.md`、`crates/xylitol-app-tui/src/AGENTS.md`、`llmanspec/AGENTS.md`、`xylitol-dev` skill 相关 reference 同步。

## Capabilities

- `protocol-app`：JSON-RPC 2.0 信封、方法名寻址、事件闭集与未知降级、握手版本（新增与替换 @req 见 `design.md`）。
- `server-core`：HTTP / WS 载体、下行通知、seq / journal / resync 语义平移、非法信封判定。
- `workspace-assembly`：跨端 Host 信封宣告 `jsonrpc`。
- `app-tui-bridge` / `app-tui-host` / `app-tui-transcript`：线上措辞与 `bash/output` 通知名（语义不变）。

## Impact

- **破坏性变更**：旧 fory-v3 客户端与 `XYLITOL_ENVELOPE_V3` 环境变量全部失效；旧二进制 `POST /rpc` 返回 400。所有端同批升级，无兼容层。
- **数据**：会话落盘格式、provider 配置、插件 ABI 均不变。
- **不在范围**：插件 ABI；OpenAPI / SSE；`x-writer-token` header 映射（任务列为可选，本次不做，`writer_token` 仅作为 `params` 字段透传）。
- **门禁**：`just qa` 新增版本命名门禁；`cargo tree -i fory` 必须为空。
