# Design — 清理 TUI 验收尾项

## 范围

纯内部清理与 lab 证据；不新增产品行为、不改 live specs。

## 详情 popup 清理

- 删除 `UiRoot` 的 `session_popup_open` / `session_popup_id` 字段与构造初始化。
- 删除 `session_popup` pending 字段及 root / host / effects 方法。
- 删除 `slot_nav` 的 popup Esc 分支、`slot_input` 的 `detailsCycle` 分支、BDD step 与测试。
- 保留 sidebar 行为不变；窄档 sidebar 请求静默。

## args_full

- `UiEntry::Tool.args_full: String`；bridge 生产路径填完整参数（与 `args_preview` 同源）。
- `paint_tool_block` 新增 `args_full` 参数；参数正文优先 `args_full`，为空回落 `args_preview`（测试构造器兼容）。
- 缓存指纹加入 `args_full`；header 仍取 `args_preview` 首行。

## CPU lab

- `cpu_jiffies_self` 在 unix 用 `libc::getrusage(RUSAGE_SELF)` 取 utime+stime（微秒），替换 `/proc`。
- 新增 `lab_ao_stream_1000_delta_cpu`：1000 次 1ms 间隔 delta，每 33 次 tick 同步 + paint，断言进程 CPU <20%。
- 保留既有 `lab_ao_stream_delta_perf_report`。
