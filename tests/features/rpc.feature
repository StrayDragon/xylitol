Feature: RPC 模式 — stdio 传输层协议

  RPC 传输层从 stdin 读取 Command JSON 行，向 stdout 写入 Event JSON 行。
  本 feature 覆盖协议层面场景。

  Scenario: Subscribe 命令在 stdio 下被拒绝
    Given RPC 传输已启动
    When 发送 Subscribe 命令
    Then 错误事件包含 requires WebSocket connection

  Scenario: ApproveTool 命令在 stdio 下被拒绝
    Given RPC 传输已启动
    When 发送 ApproveTool 命令
    Then 错误事件包含 requires WebSocket connection
