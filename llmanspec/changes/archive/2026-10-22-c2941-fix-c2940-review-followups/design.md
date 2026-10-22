# design — c2941-fix-c2940-review-followups

## 目标

修复 c2940（JSON-RPC 2.0 迁移）复核发现的 P0/P1 契约、BDD、门禁与文档缺口，附带少量不改行为的 P3 测试清理。不处理 P2 的 resync/seq/多客户端语义缺陷（另立 c2942）。

## 决策

1. **AgentEnd.messages 契约显式化（r1698）**：线上不携带该字段（wire `Event::AgentEnd` 无载荷），面本地保留；跨进程客户端以 `get_messages` 重建 transcript。落在 r1698 规则句尾，不加新场景（wire 单测已有显式非空丢弃用例）。
2. **错误码分族（r2918）**：标准类别错误 `invalid_input`→-32602、`unregistered_method`→-32601、`internal_error`→-32603；其余产品业务错误用 -32000..-32099 且 `error.data.code` 给稳定字符串（session_busy/writer_conflict/not_found 等）。与 `codec::business_error` 现有映射一致，只修 spec 表述。HTTP 200/400/413 句保持不变。
3. **server-core 旧名清理**：`rpcId` → `call_id`（两处），`v1 不得实现…` → `首版 不得实现…`（去版本标签）；导览 `（48 规则）` → `（59 规则）`。BDD 步骤文本同步改精确名；场景名/绑定不动。
4. **413 场景（r2918 下新增）**：`请求体超限判 413`（POST /rpc 超过 4 MiB 正文 → 413 + `error.data.code=payload_too_large`）。步骤走 reqwest 发 `MAX_BODY_BYTES + 1` 字节原始 body，避免构造巨大 JSON（速度快）；步骤与绑定按精确文本新增。
5. **batch 上限（r2917 + codec）**：`MAX_BATCH_ITEMS = 100`；`parse_text` 在空 batch 检查后拒绝超限，错误为 `ErrorObject::new(INVALID_REQUEST, "batch too large", "batch_too_large")`（HTTP 400 路径，经 `respond_to_body` 的 `Reply::Invalid`）。spec r2917 补一句；单测覆盖 100 放行 / 101 拒绝；不加 BDD（属信封形状，单测足够）。
6. **版本命名闸收紧**：
   - `FORBIDDEN_LITERALS` 增 `"rpcId"`（精确大小写；仓库修完后无残留）。
   - 新增 specs 版本标签检查：扫 `llmanspec/specs/**/*.feature` 行内 `\bv[0-9]+\b`；放行 `v5/v6/v7`（会话落盘格式）与行内含 `/<token>`（URL 路径如 `/api/v1`、`/v1/responses`）；否则报 `version label in spec`。修前命中 server-core `v1 不得实现…`，修后绿。
   - `ALLOWED_SEGMENTS` 收紧 `{v1,v5,v6,v7}` → `{v5,v6,v7}`（v1 仅作为 URL 段合法，由 specs 行级豁免与字符串不检查覆盖；代码标识符不得再用 v1 段）。跑闸验证；若真实外部契约误报则仅加最小豁免。`deepseek` marker 不动。
7. **文档**：根 `AGENTS.md` crate 摘要 kernel 依赖补 `serve + app-print`（与 `crates/xylitol-kernel/Cargo.toml` 一致）；`src/AGENTS.md` mermaid 补 `xylitol_kernel --> xylitol_serve` 与 `xylitol_kernel --> xylitol_app_print` 两条边。
8. **listen.rs 边界单测**：恰好 `MAX_BODY_BYTES` 字节（空白）→ 不判 413，走 400 parse error（-32700），与既有 MAX+1→413 互补。
9. **P3 清理（不改行为）**：
   - `http_ws::unary_bound` 删除死分支 `"reload_runtime"`，改用 `xylitol_protocol::registry::METHOD_RELOAD` 比对 `"reload"`。
   - wire 测试：`AgentEnd { messages: 非空 }` → `Event::AgentEnd` → 回来空（显式非空丢弃）；`MessageStart/Update/End` 往返样例补 `Some(AgentMessage)`；继续用 Debug 字符串比较，不派生 PartialEq。
   - `应答为产品错误且为未登记方法` then 步骤改严格断言（HTTP 200 + error.code=-32601 + data.code=unregistered_method）；该步骤文本被 `一元稳定错误信封` 与 `方法表未知为32601` 两场景共用，同时收紧。
   - `occupancy/host.rs`：补 `session/resources`（`push_resources`）与 question（`request_question`）不消耗 journal seq 的单测（既有用例只覆盖 approval/bash）。
   - r1693 “每个 client→core 消息” → “command-backed 方法”，避免对 host.describe/reload/loaded_resources/arm_tool_freeze/persist_trust 过度声明。
   - proposal 范围声明补 `agent-runtime`/`layer-architecture`（c2940 迁移同样触及的两个 spec/BDD 绑定面，本变更不动其语义）。

## 文件清单

- `llmanspec/specs/protocol-app/protocol-app.feature`：r1693/r1698/r2917/r2918 语句 + 新 413 场景。
- `llmanspec/specs/server-core/server-core.feature`：rpcId×2、v1×1、导览计数。
- `tests/bdd/steps_server.rs`：两处 rpcId 步骤文本改 call_id；unregistered then 步骤严格化。
- `tests/bdd/steps_protocol.rs`：新增 413 两步骤。
- `tests/bdd/bindings_protocol_app.rs`：新增 `请求体超限判 413` 绑定。
- `crates/xylitol-protocol/src/codec.rs`：`MAX_BATCH_ITEMS` + 拒绝逻辑 + 单测。
- `crates/xylitol-serve/src/listen.rs`：精确边界单测。
- `crates/xylitol-serve/src/occupancy/host.rs`：journal seq 单测扩展。
- `crates/xylitol-app-tui/src/host_client/http_ws.rs`：`unary_bound` 清理。
- `crates/xylitol-wire/src/event.rs`：测试补样例。
- `scripts/check_no_version_names.py`：`rpcId` 字面量 + specs 版本标签检查 + ALLOWED_SEGMENTS 收紧。
- `AGENTS.md`、`src/AGENTS.md`：crate 图文档。
- `llmanspec/changes/c2941-fix-c2940-review-followups/{proposal.md,design.md,tasks.md}`。

## 测试与验收

- 单测：`cargo test -p xylitol-protocol`（batch 上限）、`cargo test -p xylitol-serve --features listen`（413 边界、journal seq）、`cargo test -p xylitol-wire`、`cargo test -p xylitol-app-tui`（相关）。
- BDD：`cargo test --all-features --test bdd`（新 413 场景 + 改文本两场景必须绿）。
- 门禁：`python3 scripts/check_no_version_names.py --check` 绿（修前 server-core `v1` 被新检查命中）；`llman-sdd validate c2941-fix-c2940-review-followups --strict` 通过。
- 排版：`cargo fmt --all`、`just lint`（或定向 clippy）无新告警。
