---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c40-add-hooks"
---

```toon
kind: llman.sdd.spec
name: "hook-system"
purpose: "TBD - created by archiving change c40-add-hooks. Update purpose after archive."
requirements[3]{req_id,title,statement}:
  r1,"hook-dispatcher",System MUST implement a HookDispatcher that executes registered hooks for 10+ event types.
  r2,"three-tier-config","System MUST support global/project/user three-tier hook configuration with later layers overriding earlier."
  r3,"script-execution",System MUST execute hook scripts via stdin JSON context and parse stdout for block/allow control.
scenarios[3]{req_id,id,given,when,then}:
  r1,happy,a pre.tool_call hook is registered,tool is about to execute,hook script runs and receives event context via stdin
  r2,happy,global hook and project hook both exist for same event,hook dispatcher runs,project hook overrides global hook
  r3,happy,hook script returns block action,tool execution is pending,tool execution is blocked with reason
```
