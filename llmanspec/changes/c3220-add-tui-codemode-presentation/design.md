# Design — TUI 呈现 run_code 运行结果

## 目标

- 让 `run_code` 的外层结果、错误、截断标记与嵌套调用摘要可读。
- 不新增 `/codemode` 命令、面板或状态短标识；用户启用 codemode 并选择 `run_code` 后自动生效。
- 只读展示，不新增执行路径。

## 现状

- `run_code` 是 `ModelOnly` 工具；TUI 只按通用 tool 事件显示。
- 外层结果目前只有扁平文本；嵌套摘要需要 c3222 的结构化 `details`。
- c3106 的嵌套事件被抑制；外层 `run_code` 结果是主要可观察面。
- TUI 只经 `XyDriver` / 协议事件读取，不直调 runtime / session store。

## 决策

### 1. 不新增命令 / 状态

- 不新增 `/codemode` slash、面板或 header / status 短标识。
- codemode 是否启用、catalog 内容属于配置/诊断面，不作为用户日常交互入口。
- 不因 `run_code` 事件改变固定区布局或新增常驻文案。

### 2. run_code 外层呈现

- 工具头显示脚本摘要（首行 / 长度），不展开完整脚本。
- 结果区显示结构化输出：成功值、错误类别与消息、截断标记、耗时、嵌套调用计数/摘要。
- 嵌套摘要来自 c3222 的 `details`；不得解析 `run_code` 的 JSON 文本。
- 错误不得只显示“script error”；使用 `error_kind` + 可读消息。

### 3. 数据与边界

- TUI 只消费事件中的 `details`；缺失时回退到现有通用文本。
- 不因 details 触发工具调用；不新增第二套 slash / steer 执行路径。
- 不改变 codemode 权限、隔离、catalog 语义。

### 4. 空态

- 无 `details`：按普通 tool 结果显示，不伪造嵌套摘要。
- `details` 截断：显示截断标记，不展开超长内容。
- 嵌套调用为空：只显示脚本结果，不显示空摘要块。

## Seams 与测试

- TUI harness：有 details / 无 details；成功 / 错误 / 截断 / 嵌套摘要。
- BDD：`run_code` 外层结果渲染；不出现 `/codemode` 命令。
- 回归：无 codemode 配置时无额外状态；其他工具显示不变。

## 风险

- 嵌套事件被抑制后，外层结果必须携带足够 details；否则只能显示计数。
- details 体积过大可能挤压固定区；只显示摘要，完整内容按现有 fold 规则。
- 不同工具 details 形状不同；TUI 只识别 `run_code` 的稳定字段，未知字段忽略。

## 未决问题

- `run_code` details 的稳定字段集合：脚本摘要、耗时、截断、嵌套调用列表的最小形状。
- 嵌套参数展示沿用现有 tool 参数策略；本地 TUI 不做额外脱敏，仅按长度截断。
- 错误类别如何映射到现有 tool 错误样式；是否需要颜色区分。

## 迁移

- 不改变 `run_code` schema / guest ABI。
- 不改变现有 `/capabilities` 语义；不新增 `/codemode`。
