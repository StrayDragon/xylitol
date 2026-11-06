# Tasks

## 1. 规格

- [x] 1.1 在 `app-tui-transcript` 新增规则 `run-code-outer-presentation`：`run_code` 外层工具块按结构化 `details` 显示脚本摘要、结果、错误类别、截断标记、耗时与嵌套调用摘要；`details` 缺失回退通用文本；不得解析脚本 JSON 或伪造嵌套摘要；不得新增 `/codemode` 命令、面板或常驻状态。
- [x] 1.2 场景覆盖有 `details` 的成功展示与无 `details` 的回退展示；绑定 BDD step。

## 2. TUI 呈现

- [x] 2.1 确认 `ToolExecutionEnd.details` 已进入 bridge；`run_code` 外层块消费脚本摘要、结果、`error_kind`、截断、耗时与嵌套调用摘要。
- [x] 2.2 无 `details` 时保持现有通用文本显示；嵌套调用为空时不显示空摘要块；不新增 `/codemode` 命令、面板或状态短标识。
- [x] 2.3 其他工具显示不变；不改变 `run_code` schema、权限、隔离或执行路径。

## 3. 测试与门禁

- [x] 3.1 BDD：`run_code` 有 `details` 时工具块显示脚本摘要与嵌套摘要；无 `details` 时只显示结果文本。
- [x] 3.2 回归：其他工具显示不变；不出现 `/codemode` 命令。
- [x] 3.3 `just fmt`、相关 clippy、`just test`、`just qa`；BDD 新场景绑定并全绿。
