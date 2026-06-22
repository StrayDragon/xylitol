# Design: c185-upgrade-agent-loop

## Dual Loop Architecture

```
Outer Loop:
  while has_more:
    Inner Loop:
      while has_tool_calls OR has_steering:
        - process steering messages
        - call LLM → stream response
        - execute tool calls (sequential/parallel)
        - emit turn_end
        - prepareNextTurn / shouldStopAfterTurn
    check follow-up queue → if any, set as pending, continue outer loop
  emit agent_end
```
