# Design：裸规则 → 可执行场景

决策：不做合并/删除，全部 4 条裸规则原地升级为 `@req:` 规则 + 嵌套场景（语义不变，仅补可执行示例）。理由：四条均有单测锚点（`# verified-by: fn`），行为可程序判定；删除/合并会动既有 `规则:` 语义，超出压降必要范围。

## 场景 × seam 映射

| 规则 | 新场景 | 断言 seam |
|---|---|---|
| protocol-app r1927 | `json-depth-limit-envelope-decode` | `codec::decode_str`：>2048 层信封解码早退失败 |
| protocol-app r1927 | `json-depth-limit-admits-tree-scale` | `codec::decode_str`：约 200 层（实机深树量级）信封解码成功 |
| server-core r1928 | `non-binary-body-is-illegal-envelope` | 真实监听 `POST /rpc`：JSON 对象 / JSON 数组 / 纯文本 body 均 HTTP 400 |
| server-core r1923 | `arm-freeze-bounded-under-hanging-mcp` | `HostState::for_test_with_mcp`（`sleep 300` stdio 永挂）+ `handle_unary("arm_tool_freeze")`：门时限（15s，断言 <30s）内 `tools_table_frozen` 且 `mcp_bootstrap_complete` |
| server-core r1925 | `arm-freeze-lease-free-under-write-conflict` | 同上 + 先 `set_model` 建立租约：无租约冲突、仍返回 `tools_table_freeze` 快照 |

## 落点

- 步骤：`tests/bdd/steps_protocol.rs`（纯协议，`ProtocolBdd.last_decode`）/ `tests/bdd/steps_server.rs`（`ServerTest` 增 `nonbinary_posts`、`freeze_*` 字段；直连 host seam，不起监听——与单测同路径，先例 `steps_c2827.rs` 已用 mcp fixture 脚本）。
- 绑定：`bindings_protocol_app.rs` / `bindings_server.rs`。
- 单测保留：BDD 锁行为合约，单测锁内部探测细节（`json_nesting_exceeds_limit` 为 `pub(crate)`，不为此拓宽可见性）。

## 权衡

- 不经 HTTP 走 arm_tool_freeze：写者物化与租约建立需 host 内部 seam，HTTP 路径绕远；`handle_unary` 即产品 unary 公共入口（server-core r1776 语义）。
- 超限场景不断言「恰好 2048 边界」：边界细节由单测 `decode_rejects_beyond_json_depth_limit` 承担，BDD 只锁线入口行为。
