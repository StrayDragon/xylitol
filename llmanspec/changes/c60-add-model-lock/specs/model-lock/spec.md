---
llman_spec_valid_scope:
  - src/agent/model.rs
llman_spec_valid_commands:
  - llman sdd validate c60-add-model-lock --type spec --strict --no-interactive
llman_spec_evidence:
  - cargo test -p xylitol --lib agent::model tests pass
---

```toon
kind: llman.sdd.delta
ops[2]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,preemption-lock,"System MUST implement priority-based preemption locks on model instances with Idle/Busy/Preemptible/Preempting state machine.",null,null,null
  add_requirement,r2,checkpoint-restore,"System MUST save and restore task context when a low-priority task is preempted.",null,null,null
op_scenarios[2]{req_id,id,given,when,then}:
  r1,happy,"low-priority task holds model lock","high-priority task requests lock","lock transitions to Preemptible then Preempting then transferred"
  r2,happy,"task is preempted at step 3","high-priority task completes","original task restores from checkpoint and resumes at step 3"
```
