---
llman_spec_valid_scope:
  - src/infra/dap
llman_spec_valid_commands:
  - llman sdd validate c85-add-dap-layer --type spec --strict --no-interactive
llman_spec_evidence:
  - cargo check --features dap passes
---

```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,dap-trait,"System MUST define DapClient trait with attach set_breakpoints continue_execution get_variables get_stack_trace disconnect methods.",null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"dap feature is enabled","DapClient trait is referenced","trait compiles with no external dap backend dependency"
```
