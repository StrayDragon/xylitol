```toon
kind: llman.sdd.delta
ops[3]{op,req_id,title,statement,from,to,name}:
  add_requirement,r27,"codex-parity-keymap","System MUST provide a RuntimeKeymap abstraction (KeyBinding + context maps) and route TUI shortcuts through it, instead of matching raw KeyCode values directly.",null,null,null
  add_requirement,r28,"composer-tab-queue","System MUST implement Codex-style composer semantics where Tab queues the current draft while an agent task is running; when idle it behaves as submit except for bang-shell drafts.",null,null,null
  add_requirement,r29,"bang-shell-special-case","System MUST treat drafts starting with '!' as shell-mode drafts; when idle, Tab MUST NOT submit bang-shell drafts.",null,null,null
op_scenarios[3]{req_id,id,given,when,then}:
  r27,happy,"",user presses a configured shortcut key,the corresponding action is dispatched via RuntimeKeymap matching
  r28,happy,"",agent is running and user presses Tab,draft is queued (not submitted) and status bar queue length increments
  r29,happy,"","agent is idle, composer starts with '!',",pressing Tab does not submit or clear the draft
```
