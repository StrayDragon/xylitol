---
llman_spec_valid_scope:
  - src/interface/print.rs
llman_spec_valid_commands:
  - llman sdd validate c30-add-print-mode --type spec --strict --no-interactive
llman_spec_evidence:
  - cargo run -- --mode print "hello" outputs text to stdout
---

```toon
kind: llman.sdd.delta
ops[2]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,streaming-output,"System MUST stream agent text output to stdout in real-time in Print mode.",null,null,null
  add_requirement,r2,tool-display,"System MUST display tool execution name and result summary in Print mode.",null,null,null
op_scenarios[2]{req_id,id,given,when,then}:
  r1,happy,"agent emits TextDelta events","Print mode is active","text appears on stdout progressively"
  r2,happy,"agent calls read tool then bash tool","Print mode displays results","each tool shows [Tool: name] summary line"
```
