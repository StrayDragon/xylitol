---
llman_spec_valid_scope:
  - src/agent/loop.rs
llman_spec_valid_commands:
  - llman sdd validate c25-add-agent-loop --type spec --strict --no-interactive
llman_spec_evidence:
  - cargo test -p xylitol --lib agent::loop tests pass
---

```toon
kind: llman.sdd.delta
ops[3]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,execution-loop,"System MUST implement an agent loop that calls LLM dispatches tool calls and repeats until no more tool calls.",null,null,null
  add_requirement,r2,event-system,"System MUST emit typed events (TextDelta ToolCallStart ToolCallEnd StepComplete Error) during execution.",null,null,null
  add_requirement,r3,session-runtime,"System MUST persist session state via adk-session SQLite backend.",null,null,null
op_scenarios[3]{req_id,id,given,when,then}:
  r1,happy,"a mock LLM returns text then tool call then final text","agent loop runs","all events are emitted and loop terminates"
  r2,happy,"agent is executing a prompt","LLM streams tokens","TextDelta events are emitted for each chunk"
  r3,happy,"a session completes one turn","session is saved","state is persisted to SQLite and can be restored"
```
