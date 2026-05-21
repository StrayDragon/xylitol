---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c90-update-default-features"
---

```toon
kind: llman.sdd.spec
name: "build-config"
purpose: "TBD - created by archiving change c90-update-default-features. Update purpose after archive."
requirements[1]{req_id,title,statement}:
  r1,"Default features include all non-dev features","Cargo.toml default MUST include all features except those prefixed with dev-."
scenarios[2]{req_id,id,given,when,then}:
  r1,"all-features-on",Cargo.toml exists with updated default list,cargo build is run without extra flags,"the binary includes all non-dev feature functionality"
  r1,"minimal-build",Cargo.toml exists with updated default list,"cargo build --no-default-features --features ui-tui is run","only ui-tui and its dependencies are compiled"
```
