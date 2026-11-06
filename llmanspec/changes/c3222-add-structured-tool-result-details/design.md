# Design — 结构化工具结果 details

## 目标

- 给工具结果一个稳定的结构化通道，承载 `run_code` 的嵌套摘要、截断、错误类别。
- 保持现有工具兼容：没有 details 的工具行为不变。
- 让 TUI 只消费结构化字段，不解析工具特定的文本格式。

## 现状

- `XyTool::execute` 返回 `String`；`execute_as_parts` 返回 `Vec<AgentPart>`。
- `XyToolCallResult` 已有 `parts` / `details` / `is_error`，但只用于 dispatch 内部。
- `ToolExecutionEnd` 只有 `result: String` / `is_error: bool`；TUI 拿不到 details。
- `run_one` 计算 `details_from_parts`，只放进 `AgentMessage`，不进入事件。
- `run_code` 的 guest 结果被序列化为字符串；嵌套调用信息没有稳定结构。

## 决策

### 1. SDK 结果类型

- 新增 `XyToolOutcome`（或复用 `XyToolCallResult`）：
  - `parts: Vec<AgentPart>`
  - `details: Option<Value>`
  - `is_error: bool`
  - `error_kind: Option<XyToolErrorKind>`（可序列化）
- `XyTool` 增加默认 `execute_outcome`：
  - 默认调用 `execute_as_parts`，`details=None`，`is_error=false`。
  - 保持 `execute` / `execute_as_parts` 旧入口，避免全量迁移。
- `XyToolCallResult` 与 `XyToolOutcome` 的字段对齐；嵌套 dispatch 可复用。

### 2. 事件与 wire

- `ToolExecutionEnd` 增加 `#[serde(default, skip_serializing_if = "Option::is_none")] details: Option<Value>`。
- wire 映射透传；旧客户端忽略未知字段。
- `ToolExecutionUpdate` 不增加 details；details 只在 End 出现，避免流式重复。

### 3. run_one

- `run_one` 调 `execute_outcome`，得到 parts / details / is_error。
- after_tool 修改时，details 随 parts / is_error 一起更新；不得只改 parts。
- 事件发射使用同一份 details；AgentMessage 也使用同一份。
- hook 修改结果时保留 details，除非 hook 明确替换。

### 4. run_code details

- `RunCodeTool` 覆盖 `execute_outcome`：
  - `parts`：脚本结果文本（保持现有输出）。
  - `details`：`{ script_summary, elapsed_ms, nested_calls: [{name, args_preview, ok, error_kind, elapsed_ms}], truncated, output_bytes }`。
  - 错误：`error_kind` 取 timeout / cancelled / script / runtime / invalid_args。
- 嵌套调用摘要由 host dispatch 收集，不由 TUI 解析文本。

### 5. TUI

- 只读消费 details；缺失时回退到文本。
- 不新增执行路径；不因 details 触发工具调用。
- 截断标记来自 details.truncated，不靠字符串后缀猜测。

## Seams 与测试

- 单测：默认 outcome 适配；details 透传；after_tool 修改 details。
- 单测：run_code details 的嵌套调用、截断、错误类别。
- wire 测试：旧 payload 缺 details 仍可解析。
- TUI harness：有 details / 无 details 两种渲染。

## 示例

当前 `run_code` 外层只有字符串：

    {"parts":[{"type":"text","text":"42"}],"details":null,"is_error":false}

TUI 只能显示整段 JSON。目标结构：

    XyToolOutcome {
      parts: [text("42")],
      details: Some({
        "script_summary": "read Cargo.toml and count packages",
        "elapsed_ms": 12,
        "truncated": false,
        "nested_calls": [
          {"name":"read","args_preview":"Cargo.toml","ok":true,"elapsed_ms":3}
        ]
      }),
      is_error: false,
      error_kind: null
    }

TUI 从 `details` 读取摘要、耗时、截断与嵌套调用；不再解析 `parts` 里的 JSON 文本。

## 风险

- 事件体积增加；details 只放摘要，不放完整脚本 / 大输出；嵌套参数按现有展示策略截断，不额外脱敏。
- after_tool / hook 可能丢弃 details；需要显式合并策略。
- 旧工具不实现 outcome；默认适配必须保持行为。

## 未决问题

- 新 `XyToolOutcome` 还是复用 `XyToolCallResult`；是否需要可序列化的 `error_kind`。
- `ToolExecutionEnd.details` 是否需要 `PROTOCOL_VERSION` bump；旧客户端兼容策略。
- details 的大小 / 截断：嵌套 args 沿用现有 tool 参数展示策略；本地 TUI 不做额外脱敏。

## 迁移

- 旧工具无需改签名；逐步迁移需要 details 的工具。
- 不改变工具权限、执行顺序、取消语义。
