---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c07-fix-bdd-scenarios"
---

```toon
kind: llman.sdd.spec
name: "bdd-tests"
purpose: "TBD - created by archiving change c07-fix-bdd-scenarios. Update purpose after archive."
requirements[1]{req_id,title,statement}:
  r1,BDD steps MUST pass all 77 scenarios,"All 77 BDD scenarios MUST pass (currently 41/77, 36 failed) using rstest-bdd runner."
scenarios[1]{req_id,id,given,when,then}:
  r1,"all-pass",all 77 scenarios exist in feature files,"cargo test --test bdd -- --test-threads=1","all 77 pass, 0 fail"
```
