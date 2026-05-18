---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c70-add-session-snapshot"
---

```toon
kind: llman.sdd.spec
name: "session-persistence"
purpose: "TBD - created by archiving change c70-add-session-snapshot. Update purpose after archive."
requirements[2]{req_id,title,statement}:
  r1,"snapshot-ops",System MUST support snapshot restore spawn list prune diff and merge operations on immutable session snapshots.
  r2,compaction,System MUST automatically compact context when conversation token count exceeds configured window threshold.
scenarios[2]{req_id,id,given,when,then}:
  r1,happy,a snapshot exists,spawn is called with snapshot_id and new prompt,new agent instance starts with inherited context from snapshot
  r2,happy,conversation exceeds 75% of context window,compaction is triggered,older turns are summarized and replaced with compact system message
```
