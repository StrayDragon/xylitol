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
requirements[6]{req_id,title,statement}:
  r1,"faux-provider","System MUST provide FauxProvider that returns pre-configured response steps without network calls for deterministic agent testing."
  r2,"test-harness","System MUST provide TestHarness Builder that wires all dependencies as mock or in-memory implementations for full-stack agent testing."
  r3,"vt100-backend","System MUST provide VT100Backend feature-gated terminal emulator for pixel-accurate TUI rendering verification via insta snapshots."
  r4,"temp-file-raii","All tests that create temporary files MUST use RAII-based cleanup (tempfile crate) and MUST NOT leave artifacts after execution."
  r5,"async-test-timeout",Async integration tests MUST wrap their body with a timeout (default 10s) to prevent CI hangs on deadlock or mock failure.
  r6,"no-fixed-tmp-paths","Tests MUST NOT use hardcoded /tmp paths with fixed names; MUST use unique auto-generated paths to support parallel execution."
scenarios[6]{req_id,id,given,when,then}:
  r1,happy,FauxProvider is configured with text and tool call responses,agent loop runs with FauxProvider,all responses are returned without network calls and call_count is tracked
  r2,happy,HarnessBuilder is configured with mock responses and tools,build() is called,TestHarness contains wired AgentSession with captured events
  r3,happy,VT100Backend is created at 80x24,a ratatui widget is rendered,screen_contents() returns accurate text matching the rendered output
  r4,"lsp-cleanup",LSP test creates a temp .rs file,test completes (success or panic),temp file is automatically deleted
  r5,"timeout-deadlock",async test encounters a deadlocked mock,timeout fires at 10s,test fails with timeout error instead of hanging
  r6,"parallel-safe",two instances of config loader tests run in parallel,both use unique temp directories,no file collision or test interference
```
