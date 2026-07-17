# language: en
# migrated from tests/features/approval.feature
Feature: server-reverse-rpc
  Scenario: approve-roundtrip
    Given 服务端和已连接的 WebSocket 客户端
    When agent 执行需要审批的工具
    Then 客户端收到带有 call_id 的审批请求
    When 客户端发送 ApproveTool approved=true
    Then 工具执行继续
    And turn 正常结束

  Scenario: tool-denied
    Given 服务端和已连接的 WebSocket 客户端
    When agent 执行需要审批的工具
    And 客户端发送 ApproveTool approved=false
    Then 工具被拒绝
    And turn 继续但不包含工具结果
