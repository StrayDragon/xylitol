---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c25-add-agent-loop"
---

```toon
kind: llman.sdd.spec
name: agent-runtime
purpose: TBD - created by archiving change c25-add-agent-loop. Update purpose after archive.
requirements[3]{req_id,title,statement}:
  r1,execution-loop,System MUST implement an agent loop that calls LLM dispatches tool calls and repeats until no more tool calls.
  r2,event-system,System MUST emit typed events (TextDelta ToolCallStart ToolCallEnd StepComplete Error) during execution.
  r3,session-runtime,System MUST persist session state via adk-session SQLite backend.
scenarios[3]{req_id,id,given,when,then}:
  r1,happy,a mock LLM returns text then tool call then final text,agent loop runs,all events are emitted and loop terminates
  r2,happy,agent is executing a prompt,LLM streams tokens,TextDelta events are emitted for each chunk
  r3,happy,a session completes one turn,session is saved,state is persisted to SQLite and can be restored
```
