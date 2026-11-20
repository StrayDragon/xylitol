# Tasks

单 task 上限约 2 小时；全部无行为变更（错误 code/文案、defer 时机、MCP OAuth 行为不变）。T1-T3 为载荷类型化垂直切片，T4 收编 defer，T5 为 gateway 证伪落地，T6 全量门禁。

## T1 subscribe / prompt 两臂载荷类型化（先行）

- [x] host.rs 新增私有 `SubscribePayload`（`last_seq: Option<u64>`）与 `PromptPayload`（`message` / `contexts` / `model_id` / `thinking_level` 均 Option，`#[serde(default)]`）。
- [x] :1699-1756 两臂改 `serde_json::from_value` 单点解析；校验逻辑与文案逐字保留（`"subscribe requires session_id"` / `"missing message"`）；`contexts` 解析失败回落空 Vec（等价现状 `.ok().unwrap_or_default()`）；`model_id` / `thinking_level` 保留 trim + 非空过滤。
- [x] 单测补齐：缺键默认、空 message 报错、contexts 非法 JSON 回落空、model_id trim 过滤（host.rs tests mod；:3530 / :3548 既有断言保持绿）。
- 判据：`cargo nextest run -p xylitol-serve` 全绿；`rg -Uc 'payload\s*\.get\(' crates/xylitol-serve/src/occupancy/host.rs` 23 → 18。

## T2 mcp / oauth 臂载荷类型化

- [x] 新增 `McpResourceReadPayload` / `McpPromptGetPayload` / `McpOAuthStartPayload` / `McpOAuthCancelPayload` / `McpOAuthStatusPayload`（字符串字段 Option → 缺省 ""，等价 `unwrap_or("")`；`arguments` 缺省 `Value::Null`）。
- [x] :1877-1940 各臂改单点解析；`"unavailable"` / `"no writer engine"` 与 driver 调用顺序不变。
- 判据：`cargo nextest run -p xylitol-serve` 全绿；rg 计数 18 → 10。

## T3 trust-mode 臂 + export-import staging + defer 两参复用

- [x] :1841-1848 trust mode 解析改 typed；未知 mode 仍报 `"unknown trust mode: {other}"`。
- [x] stage_payload（:1997-2005 起）的 output_path / input_path / content 判定改 typed；null 与缺键等价。
- [x] defer_runtime_setting :1381-1385 / :1413-1417 改用与 idle 臂同形状的 `SetModelPayload` / `SetThinkingLevelPayload`；`"model not found: {id}"` / `"empty thinking level"` 文案不变。
- 判据：`cargo nextest run -p xylitol-serve` 全绿；`rg -Uc 'payload\s*\.get\(' crates/xylitol-serve/src/occupancy/host.rs` = 0（ws.rs:153/:156 两处不在范围）。

## T4 PendingRuntimeSetting 收编 defer

- [x] 新增 `PendingRuntimeSetting`（两个 Mutex 字段 + `set_model` / `set_thinking` / `take_all` / `clear` 方法），替换 host.rs:95-96 两字段与 :504-533 / :1390 / :1407 / :1422 / :1763 调用点；`flush_pending_runtime` 与 `flush_pending_runtime_for_test`（:539-541，c3402 C17 挂门）签名不变。
- [x] 补 busy defer 三臂直接单测（复用 fixture 先例 `materialize_writer` + `mark_run_inflight_for_test`）：set_model / cycle_model / set_thinking_level busy 时 driver 值不变、pending 收录；flush 后 driver 值生效（含给既有 :1812-1843 测试补 flush 生效断言）。
- 判据：`cargo nextest run -p xylitol-serve` 全绿且新增 ≥4 测试；`rg -n 'pending_model|pending_thinking' crates/xylitol-serve/src` 0 命中。

## T5 gateway 零引用证明落地（证伪，不删除）

- [x] 复跑 design「删除验证」两条 rg 命令并记录输出：getter 生产命中 5 处（client.rs:69/:1051-1055/:1204/:1324/:1357）→ 按 proposal 条件句不删 setter/getter/写入点/HostOAuthGateway。
- [x] 核验 `impl Default for HostOAuthGateway`（oauth.rs:40-44）：全仓无 `HostOAuthGateway::default` 调用，但删除触发 clippy `new_without_default` → 保留。
- [x] 提交说明附证伪输出，与 design「删除验证」结论一致。
- 判据：两条 rg 命令输出与 design 记录一致；本任务零生产代码改动；`cargo clippy -p xylitol-serve --all-targets -- -D warnings` 绿。

## T6 全量门禁

- [x] `just fmt` + `just lint`。
- [x] `cargo nextest run -p xylitol-serve -p xylitol-plugin-sdk` 全绿，记录输出。
- [x] `just qa` 全绿，记录输出。
- [x] `llman-sdd validate c3405-refactor-serve-occupancy-host` failed 0。
