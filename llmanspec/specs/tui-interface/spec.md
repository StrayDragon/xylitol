---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c80-add-tui"
---

```toon
kind: llman.sdd.spec
name: "tui-interface"
purpose: "TBD - created by archiving change c80-add-tui. Update purpose after archive."
requirements[2]{req_id,title,statement}:
  r1,"ratatui-components",System MUST implement ~25 ratatui TUI components for chat streaming tool output diff preview and approval flow.
  r2,"event-driven-ui","System MUST subscribe to agent event stream and update TUI in real-time."
scenarios[2]{req_id,id,given,when,then}:
  r1,happy,TUI mode is activated,interface renders,all major components render without panic
  r2,happy,agent emits TextDelta events,TUI is running,chat component updates progressively
```
