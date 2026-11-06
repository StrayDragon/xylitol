# Design — TUI codemode 呈现

## 目标

- 让用户看见 codemode 已启用、catalog 内容、inline / deferred 工具与 tool-exposure 过滤结果。
- 让 `run_code` 外层结果、错误、截断标记与嵌套调用摘要可读。
- 只读展示，不新增执行路径；不新增 header / status 短标识。

## 现状

- `run_code` 是 `ModelOnly` 工具；TUI 只按通用 tool 事件显示。
- 外层结果目前只有扁平文本；嵌套摘要需要 c3222 的结构化 details 通道。
- `/capabilities` 只覆盖 session 工具与 MCP，不覆盖 codemode。
- c3106 的嵌套事件被抑制；外层 `run_code` 结果是主要可观察面。
- TUI 只经 `XyDriver` / 协议快照读取，不直调 runtime / session store。

## 决策

### 1. 数据源

- 新增只读 `CodemodeStatus` 快照：enabled、runtime、mode、timeout、fuel、memory、catalog 数量、inline / deferred 工具、tool-exposure 过滤结果。
- 由 runtime / kernel 生成，经现有 `XyDriver` 或协议 unary 暴露；TUI 不直接读 runtime。
- 快照缺失或 codemode 未启用时，面板显示“未启用”，不报错。

### 2. `/codemode` 面板

- 只读 SelectList / 信息面板；不提供执行按钮。
- 字段分组：运行限制（runtime / mode / timeout / fuel / memory）、catalog（总数 / inline / deferred / 被过滤）、工具列表（名字 / namespace / exposure）。
- 关闭面板不 abort agent；不改变当前 turn。

### 3. `run_code` 外层呈现

- 工具头显示脚本摘要（首行 / 长度），不展开完整脚本。
- 结果区显示结构化输出：成功值、错误、截断标记、耗时 / 嵌套调用计数。
- 嵌套调用摘要来自 c3222 的结构化 details；不得依赖被抑制的嵌套事件，也不得解析 `run_code` 的 JSON 文本。
- 错误显示错误类别与可读消息；不得只显示“script error”。

### 4. 只读边界

- 不新增第二套 slash / steer 执行路径。
- 不因打开面板触发 run_code 或工具调用。
- 不改变 codemode 权限、隔离、catalog 语义。

### 5. 空态与未启用

- codemode 未启用：`/codemode` 显示未启用；不显示空 catalog 误导。
- catalog 为空：显示 0 工具与原因（未配置 / 全部被过滤）。
- 声明缺失：显示诊断，不伪造工具。

## Seams 与测试

- TUI harness：启用 / 未启用面板快照；inline / deferred / filtered 计数；关闭面板不 abort。
- BDD：`run_code` 成功、错误、截断、嵌套摘要。
- 回归：无 codemode 配置时无面板入口、无额外状态。

## 风险

- `CodemodeStatus` 跨进程快照需要协议字段；避免把大 catalog 全量塞进每个事件。
- 嵌套事件被抑制后，外层结果必须携带足够 details；否则只能显示计数。
- 面板字段过多会挤压固定区；用分组与折叠，不新增常驻 header 文案。

## 未决问题

- `CodemodeStatus` 走 `LoadedResourcesSnapshot` 扩展还是新 unary；前者无协议 bump，后者需 registry / wire。
- 未启用时如何区分“配置关闭”与“插件未装配”；是否需要 config 快照。
- catalog 列表大小与截断策略；面板只显示计数 + 按需列表，还是全量快照。

## 迁移

- 不改变 `run_code` schema / guest ABI。
- 不改变现有 `/capabilities` 语义；`/codemode` 独立。
