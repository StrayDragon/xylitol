# 设计：arm_tool_freeze 有界决议（拆 TUI↔runtime 双向死锁）

## 死锁环（实机确认）

- serve 配置 MCP（context7 SSE / lspz stdio）但当前环境连不上 → `mcp_boot=Running(connecting)` 永驻。
- TUI 首轮 submit → `!is_tools_frozen()` → arm + "Assembling" 门；门只在 `is_tools_frozen()`（tools_table_frozen）翻转后开。
- tools_table_frozen 只由 runtime `ensure_tool_table_frozen()`（带 15s 时限、超时 detach+freeze）翻转——但它在 `run()` 里，而 run 等门开 → 死锁：provider 请求永不发出、spinner 永转。

## 方案

`arm_tool_freeze` unary handler（host.rs）从「arm+单次 poll 即回快照」改为：
- 调 `driver.ensure_tool_table_frozen()`（既有有界机制：settle/超时 detach/freeze armed 子集 + gate notice）。
- 返回权威快照：`tools_table_frozen=true`、`mcp_bootstrap_complete=true` → TUI 门有界打开、turn 照跑（工具降级）。
- 这是「权威决议点」：unary 的契约从「询问状态」改为「有界决议」，与 run-loop 门（host.rs:1170）同构。

## 明确不做

- 不加客户端侧第二个超时（unary 已权威有界，客户端无需重复）。
- 不改 MCP connect 层（超时是门层决策，connect 层可后续再议）。
