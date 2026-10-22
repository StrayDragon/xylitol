# tasks — c2941-fix-c2940-review-followups

## P0 契约修复

- [x] T1 `protocol-app` r1698 规则句尾补 `AgentEnd` 的 `messages` 面本地保留、线上不携带、`get_messages` 重建 transcript。
- [x] T2 `protocol-app` r2918 错误码句改写：标准类别（invalid_input→-32602、unregistered_method→-32601、internal_error→-32603）与业务错误（-32000..-32099 + 稳定 data.code）分族；HTTP 200/400/413 句保留。
- [x] T3 `server-core`：`rpcId 为 xyz`→`call_id 为 xyz`、`server 将 rpcId 标记为 expired`→`server 将 call_id 标记为 expired`、`v1 不得实现`→`首版 不得实现`、导览 `（48 规则）`→`（59 规则）`；同步 `tests/bdd/steps_server.rs` 两处步骤文本（场景名/绑定不变）。
- [x] T4 `protocol-app` r2918 下新增场景 `请求体超限判 413`；`tests/bdd/steps_protocol.rs` 新增精确步骤（reqwest 发 >4 MiB 原始字节，断言 413 + data.code=payload_too_large）；`tests/bdd/bindings_protocol_app.rs` 加绑定。

## P1 健壮性 / 门禁 / 文档

- [x] T5 `crates/xylitol-protocol/src/codec.rs`：`pub const MAX_BATCH_ITEMS: usize = 100`；`parse_text` 空 batch 检查后拒绝超限（-32600 / batch_too_large）；单测 100 放行、101 拒绝；`protocol-app` r2917 补 batch 上限语句。
- [x] T6 `scripts/check_no_version_names.py`：`FORBIDDEN_LITERALS` 加 `"rpcId"`；新增 specs `.feature` 行级 `\bv[0-9]+\b` 检查（豁免 v5/v6/v7 与行含 `/<token>`）；`ALLOWED_SEGMENTS` 收紧为 `{v5,v6,v7}`；跑 `--check` 验证修前命中、修后绿。
- [x] T7 文档：根 `AGENTS.md` crate 摘要 kernel 依赖补 `+ serve + app-print`；`src/AGENTS.md` mermaid 补 `xylitol_kernel --> xylitol_serve`、`xylitol_kernel --> xylitol_app_print`。
- [x] T8 `crates/xylitol-serve/src/listen.rs`：恰好 `MAX_BODY_BYTES` 字节空白 body → 非 413（400 parse error）单测。

## P3 轻量清理（不改行为）

- [x] T9 `http_ws.rs` `unary_bound`：删 `"reload_runtime"` 死分支，用 `registry::METHOD_RELOAD`。
- [x] T10 `crates/xylitol-wire/src/event.rs` 测试：`AgentEnd` 非空 messages 显式丢弃；MessageStart/Update/End 往返样例补 `Some(message)`（Debug 比较，不派生 PartialEq）。
- [x] T11 `tests/bdd/steps_server.rs`：`应答为产品错误且为未登记方法` 严格断言（200 / -32601 / unregistered_method）。
- [x] T12 `crates/xylitol-serve/src/occupancy/host.rs`：`session/resources` 与 question 不消耗 journal seq 单测（扩展既有 non_event 用例或新增）。
- [x] T13 `protocol-app` r1693：`每个 client→core 消息` → `command-backed 方法`。
- [x] T14 proposal 范围声明补 `agent-runtime`/`layer-architecture`。

## 验证

- [x] T15 `cargo fmt --all`、`just lint`（或定向 clippy）、`cargo test -p xylitol-protocol`、`cargo test -p xylitol-serve --features listen`、`cargo test -p xylitol-wire`、`cargo test --all-features --test bdd` 全绿。
- [x] T16 `llman-sdd validate c2941-fix-c2940-review-followups --strict` 与 `python3 scripts/check_no_version_names.py --check` 通过。
- [x] T17 分支上按 Conventional Commits 提交；不 finalize/不 push；工作区净。
