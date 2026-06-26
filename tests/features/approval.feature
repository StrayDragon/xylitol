Feature: Tool approval via reverse RPC

  When a tool with approval_required is executed, the server pushes an
  ApprovalRequired event to connected clients and waits for a response.

  Scenario: Tool approval round-trip
    Given a server with a session and a connected WebSocket client
    When the agent executes a tool that requires approval
    Then the client receives a ReverseRpc approval request with a call_id
    When the client sends ApproveTool with approved=true
    Then the tool execution proceeds
    And the turn continues normally

  Scenario: Tool denial
    Given a server with a session and a connected WebSocket client
    When the agent executes a tool that requires approval
    And the client sends ApproveTool with approved=false
    Then the tool is denied
    And the turn continues without the tool result
