---
llman_spec_valid_scope:
  - src/interface/cli
llman_spec_valid_commands:
  - llman sdd validate c15-add-cli --type spec --strict --no-interactive
llman_spec_evidence:
  - cargo run -- --help succeeds
---

```toon
kind: llman.sdd.delta
ops[2]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,clap-args,"System MUST parse CLI arguments via clap derive including mode/config/project/model/yolo options.",null,null,null
  add_requirement,r2,mode-dispatch,"System MUST dispatch to Print/Interactive/ACP mode based on --mode flag or default to Print.",null,null,null
op_scenarios[2]{req_id,id,given,when,then}:
  r1,happy,"","xylitol --mode print --config ./test.yaml is run","args are parsed with mode=Print and config path set"
  r2,happy,"no --mode flag is provided","CLI starts","Print mode is selected as default"
```
