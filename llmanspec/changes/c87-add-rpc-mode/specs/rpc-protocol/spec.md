---
llman_spec_valid_scope:
  - src/interface/rpc.rs
llman_spec_valid_commands:
  - llman sdd validate c87-add-rpc-mode --type spec --strict --no-interactive
llman_spec_evidence:
  - cargo test -p xylitol --lib interface::rpc tests pass
---

```toon
kind: llman.sdd.delta
ops[2]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,json-rpc-stdio,"System MUST implement JSON-RPC 2.0 server over stdio for IDE integration.",null,null,null
  add_requirement,r2,event-streaming,"System MUST stream agent events (text deltas and tool calls) as JSON-RPC notifications.",null,null,null
op_scenarios[2]{req_id,id,given,when,then}:
  r1,happy,"IDE sends agent/prompt request via stdin","RPC server processes it","agent starts and response is sent back via stdout"
  r2,happy,"agent emits TextDelta events in RPC mode","events are serialized","JSON-RPC notification with method agent/text_delta is written to stdout"
```
