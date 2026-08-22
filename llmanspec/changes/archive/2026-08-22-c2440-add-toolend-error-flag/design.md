# Design

## 根因

`to_wire_event` 以 `XyEvent::ToolExecutionEnd { id, name, result, .. }` 构造 `Event::ToolEnd`——is_error 被 `..` 吞掉；`try_from(&Event)` 反向硬编码 `is_error: false`。进程内（print）不受影响；attach 全量受影响。

## 裁决

- 字段名 `is_error`，`#[serde(default)]`：旧 journal / 宽松客户端按成功解析，与冷恢复「journal 非实况」语义一致。
- 不升 PROTOCOL_VERSION：加字段为加法演进，旧客户端忽略即可；host/TUI 同二进制分发。
- bridge 追加条件：`is_error && !is_mcp_tool_name && !output.contains(result)` —— MCP 错误体由专用 body 呈现；已含则不重复。

## 流程教训（记录）

本票实现先行于 Branch binding，违反 Git-native 顺序；以 skip_specs_landing 补记收编。同类 wire 变更后续 MUST 先 start 再落 main 之外。
