Feature: RPC mode — stdio transport over protocol vocabulary

  The RPC transport reads Command JSON lines from stdin and writes Event
  JSON lines to stdout. This feature covers protocol-level scenarios.

  Scenario: Subscribe command is rejected over stdio
    Given the RPC transport is started
    When a Subscribe command is sent
    Then an error event is returned indicating "requires WebSocket connection"

  Scenario: ApproveTool command is rejected over stdio
    Given the RPC transport is started
    When an ApproveTool command is sent
    Then an error event is returned indicating "requires WebSocket connection"
