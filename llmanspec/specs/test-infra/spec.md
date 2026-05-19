---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c88-add-test-infra"
---

```toon
kind: llman.sdd.spec
name: "test-infra"
purpose: "TBD - created by archiving change c88-add-test-infra. Update purpose after archive."
requirements[3]{req_id,title,statement}:
  r1,"faux-provider","System MUST provide FauxProvider that returns pre-configured response steps without network calls for deterministic agent testing."
  r2,"test-harness","System MUST provide TestHarness Builder that wires all dependencies as mock or in-memory implementations for full-stack agent testing."
  r3,"vt100-backend","System MUST provide VT100Backend feature-gated terminal emulator for pixel-accurate TUI rendering verification via insta snapshots."
scenarios[3]{req_id,id,given,when,then}:
  r1,happy,FauxProvider is configured with text and tool call responses,agent loop runs with FauxProvider,all responses are returned without network calls and call_count is tracked
  r2,happy,HarnessBuilder is configured with mock responses and tools,build() is called,TestHarness contains wired AgentSession with captured events
  r3,happy,VT100Backend is created at 80x24,a ratatui widget is rendered,screen_contents() returns accurate text matching the rendered output
```
