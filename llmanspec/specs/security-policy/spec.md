---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c50-add-security"
---

```toon
kind: llman.sdd.spec
name: "security-policy"
purpose: "TBD - created by archiving change c50-add-security. Update purpose after archive."
requirements[2]{req_id,title,statement}:
  r1,"declarative-rules",System MUST enforce declarative security rules for bash/filesystem/network access before any tool execution.
  r2,"tighten-only","System MUST only allow three-tier config overrides to tighten rules never to relax them."
scenarios[2]{req_id,id,given,when,then}:
  r1,happy,"bash tool called with rm -rf /","security policy has forbidden pattern for rm -rf",tool call is blocked and tool_call_blocked event emitted
  r2,happy,user config tries to allow a forbidden pattern,config is merged,the forbidden pattern remains blocked
```
