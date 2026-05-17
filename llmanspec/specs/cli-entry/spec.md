---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c15-add-cli"
---

```toon
kind: llman.sdd.spec
name: cli-entry
purpose: TBD - created by archiving change c15-add-cli. Update purpose after archive.
requirements[2]{req_id,title,statement}:
  r1,clap-args,System MUST parse CLI arguments via clap derive including mode/config/project/model/yolo options.
  r2,mode-dispatch,System MUST dispatch to Print/Interactive/ACP mode based on --mode flag or default to Print.
scenarios[2]{req_id,id,given,when,then}:
  r1,happy,"",xylitol --mode print --config ./test.yaml is run,args are parsed with mode=Print and config path set
  r2,happy,no --mode flag is provided,CLI starts,Print mode is selected as default
```
