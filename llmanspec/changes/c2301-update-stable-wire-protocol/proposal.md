---
depends_on:
  - c2300-update-cs-capability-split
---

# 产品位：稳定线协议

Command/Event 是 TUI ↔ host 的唯一产品真源。载体可变，语义不可双轨。证据：`c2280` `03-paths`、c2300。

## Why

现 wire 未覆盖全部产品语义，旁路与「进程内 vs 远程」会分叉。本票钉闭集。

## What Changes

- 一切产品语义都是 Command 或 Event。embed 与 attach 同一套。
- 0.0.1 前禁双语义。REST 不承载产品语义。
- 面本地不进协议：剪贴板（含 OSC 52）、TTY、`$EDITOR`、键位、绘制。
- 命令族：运行、中止、模型 / thinking、会话生命周期、导出（回传内容）、导入（接收内容）、steer / follow-up / 队列、reload、信任、工作区 bash（`!` / `!!` 与模型工具分两条）、审批与问卷应答。
- 事件族：文本 / thinking 增量；模型工具 `ToolStart` / `ToolExecutionUpdate` / `ToolEnd`；人 bang 独立增量 + `BashResult`（不并进工具流）；回合生命周期、反向 RPC、会话状态。
- 每条消息归属 session。可按序号订阅 / 续传；缓冲满则 `ResyncRequired`。`Subscribe` 之后 host 发 `ServerHello { protocol: 1 }`；对不上则断开，不降级。
- 反向 RPC：host 问客户端；第一应答生效。
- 一 session 一写者。只读 attach 上写 → 「有其他 TUI 已连接，当前仅只读」。协议能表达每 session 谁在写、谁只读。

## 非目标

字段 / 命名 / codec 切片。编码不二进制化。多会话实现、进线、符合性闸、ACP（c2302–c2305）。
