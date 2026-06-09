---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c05-rebuild-core"
---

```toon
kind: llman.sdd.spec
name: compaction
purpose: "TBD - created by archiving change c05-rebuild-core. Update purpose after archive."
requirements[6]{req_id,title,statement}:
  c1,"token-estimation",System MUST estimate context window usage from current messages using a configurable token estimator.
  c2,"should-compact",System MUST trigger compaction when estimated token usage exceeds a configurable threshold percentage of the context window.
  c3,"compact-summarize",Compaction MUST summarize older messages into a CompactionEntry while preserving the N most recent turns.
  c4,"compact-session",Compaction MUST persist the CompactionEntry to the session file and update the session tree.
  c5,"branch-summary","When navigating to a session tree branch, System MUST generate a branch summary entry bridging the context gap."
  c6,"bdd-compaction",BDD tests under tests/features/compaction.feature MUST all pass.
scenarios[6]{req_id,id,given,when,then}:
  c1,estimate,messages contain 1000 tokens of text,estimateTokens is called,result is approximately 1000 tokens
  c2,trigger,tokens exceed 80% of context window,shouldCompact is called,returns true
  c3,summarize,session has 50 turns,compact is called,first 40 turns are summarized into one CompactionEntry
  c4,persist,compaction completes,session is loaded,CompactionEntry is present with summary and cut point
  c5,branch,user navigates to earlier branch point,branch summary is generated,summary entry bridges the context gap
  c6,"bdd-pass",BDD runner invoked,"cargo test --test bdd",all compaction scenarios pass
```
