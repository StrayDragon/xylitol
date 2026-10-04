---
depends_on: [c2834]
---

# 设计：线协议 v3 硬切收尾

真值链：c2834 已落地（fbs 真源 + 生成物 + 服务端/客户端双轨 + 对拍 + TUI 产品面 v3）。
本 change 只做**减法与终态**：去 jsonrpsee、载体定位、版本 bump、死类型清理、租约窗口专项。

## 1. 决策表

| # | 决策 | 理由与代价 |
|---|---|---|
| D1 | JSON-RPC 分发**手写**在 `app::server::rpc_module`，jsonrpsee 依赖移除；`HostState::handle_unary` 仍是唯一执行面 | jsonrpsee 在本项目只承担「JSON 文本 ↔ 方法表」这层薄壳（`raw_json_request` + `RpcModule` 注册）；`codec.rs` 已有 `jsonrpc_response` / `jsonrpc_method_not_found` 作为终态形状真值。少一层依赖，语义等价项可逐条测 |
| D2 | 等价性判据锁定四项：① `id` **逐字回显**（保留原 JSON 类型，数字不进字符串）② 未登记方法 `-32601` + `data.code = unregistered_method` ③ 非法信封 `-32600` ④ 请求体 4 MiB 上限 | 这四项是 jsonrpsee 目前实际提供的全部行为（其余由 `polish_rpc_json` / `handle_unary` 承担，本就在我方）。4 MiB 来自旧 `raw_json_request(_, 4MB)` |
| D3 | 载体定位：binary 帧 = 产品上行；JSON 文本（POST body 与 WS text）= **调试通道**，与产品路径共用同一 dispatch 与同一方法表 | 保住 `/openapi.json` + `/docs`（Scalar）调试入口与双轨对拍样本；不引入第二套词表，不破坏 r1908 对拍纪律。文本通道仍 MUST 走同一 dispatch，MUST NOT 分叉语义 |
| D4 | `PROTOCOL_VERSION` 2→3；attach 预检保持硬等值（不等即致命，不降级、不重试风暴） | v3 是新的产品载体（r1902/r1911），版本号是 attach 的第一道闸；`oapi.rs` 文案与 in_process 路径都读同一常量，单点真值 |
| D5 | 四象限死变体删除：`RpcMessage::ServerHello`（握手已由 `host.describe` 承担）、`RpcMessage::ClientResponse`（无生产引用） | 变体存在即诱导客户端实现旧形状；`method.rs` 注释已声明握手不在 mux 首帧 |
| D6 | 租约跨连接窗口：HTTP 每次 POST 都是新连接 ⇒ 连接不是写者身份，令牌靠 `X-Writer-Token` 显式回显；WS 内连接本地租约仅在同一条连接内有效。三条观测点由测试钉死（见 task 3.2） | 现状语义（r1793）不变，本任务只补「窗口」专项证据：first-wins、抢约冲突、同连接续用 |

## 2. 分层落点（谁改什么）

| 文件 | 改动 |
|---|---|
| `src/app/server/rpc_module.rs` | 去 jsonrpsee：`dispatch_raw(&Arc<HostState>, …)` 手写解析 + 方法表命中（`registry::REGISTRY` ∪ `{approve_tool, answer_question}`）；task-local `CALL`（rpc_id / 入参租约 / 出参租约）形态不变 |
| `src/protocol/wire/codec.rs` | `jsonrpc_response` 收 `&Value` 形式的 id 以逐字回显；新增 `jsonrpc_invalid_request`（-32600）；`jsonrpc_method_not_found` 形状保持 |
| `src/app/server/http.rs` / `ws.rs` | 调用点从 `ProductRpc` 改为 `Arc<HostState>`；`polish_rpc_*` 的 `-32601` 补码逻辑保留（对旧 host 兼容），租约 header/extra member 逻辑不动 |
| `src/protocol/wire/envelope.rs` | `PROTOCOL_VERSION = 3`；删两个死变体 |
| `Cargo.toml` | 删 `jsonrpsee` 依赖与 `server` feature 里的 `dep:jsonrpsee` |
| `llmanspec/specs/**` | 终态措辞改写（见 §3） |

## 3. spec 收口映射（旧条款 → 终态）

| 旧 @req | 终态要点 |
|---|---|
| server-core r1778 / r1796 | 产品入口仍是 `POST /rpc` + `WS /rpc`（同一方法表）；MUST 承载两种载体：binary = v3 产品帧，JSON text = 调试通道；`/healthz`、`/openapi.json`、`/docs` 仍为调试 |
| server-core r1803 | 下行 MUST 为「与 JSON-RPC notification 同构」的帧：binary 走 v3 `ServerNotification`，调试通道走 JSON 文本；`seq`/journal/resync/固定区语义不变 |
| server-core r1804 | WS MUST 只接受 v3 binary 与 JSON-RPC 文本两类上行（外加 ping/pong/close）；其余 MUST 拒 |
| server-core r1809 | 事件推送 MUST 携带会话单调 seq；载体由协商决定（binary 优先），MUST NOT 用 SSE 作下行真源 |
| protocol-app r1696 / r1701 / r1709 | 「产品信封 MUST 为 JSON-RPC 2.0」→ 终态：产品信封 MUST 为 v3 二进制帧（协商所得）；JSON-RPC 2.0 形状是调试通道与 v3 内部载荷（`RawOk`）的原文形态；id/rpc_id 回显、`error.data.code` 产品码、单一方法表三条不变 |
| server-core r1902 / r1909 | 迁移期条款收口：双轨已完成对拍，JSON 路径降为调试通道；「v3 MUST NOT 成为唯一产品路径」改为「调试通道 MUST 与产品路径同 dispatch，MUST NOT 分叉语义」 |

## 4. 对拍与回归纪律

- 双轨对拍测试（`dual_rail_*`、`steps_wire_v3`）MUST 全绿；用例数相对 merge-base **只增不减**。
- 每条等价判据（D2 四项）MUST 有独立单测，不靠 HTTP 层聚合测试兜底。
- 租约窗口三条观测点（D6）MUST 在同一测试里按时间序断言，避免并行编号干扰。
- 前后期：门禁 `just qa`（含 `test-tui`）+ `just lint-all` + `just doc-check`；spec 改后 `llman-sdd validate --specs`。

## 5. 退路

- 若手写分发在某个 JSON-RPC 边角（如 id 为 `null` 的通知、数字精度）与 jsonrpsee 观测不一致：以 `codec.rs` 单测为准逐条钉死，MUST NOT 同时保留两套实现。
- 若 `PROTOCOL_VERSION` bump 导致 attach 预检大面积红：先确认是否仅常量未同步（in_process / oapi / 测试各自读同常量），再判是否真要回退版本号。
