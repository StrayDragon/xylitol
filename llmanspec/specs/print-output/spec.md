---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c30-add-print-mode"
---

```toon
kind: llman.sdd.spec
name: print-output
purpose: TBD - created by archiving change c30-add-print-mode. Update purpose after archive.
requirements[2]{req_id,title,statement}:
  r1,streaming-output,System MUST stream agent text output to stdout in real-time in Print mode.
  r2,tool-display,System MUST display tool execution name and result summary in Print mode.
scenarios[2]{req_id,id,given,when,then}:
  r1,happy,agent emits TextDelta events,Print mode is active,text appears on stdout progressively
  r2,happy,agent calls read tool then bash tool,Print mode displays results,"each tool shows [Tool: name] summary line"
```
