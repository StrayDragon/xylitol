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
requirements[10]{req_id,title,statement}:
  r1,"snapshot-ops",System MUST support snapshot restore spawn list prune diff and merge operations on immutable session snapshots.
  r2,compaction,System MUST automatically compact context when conversation token count exceeds configured window threshold.
  s1,"jsonl-storage","SessionManager MUST persist sessions as JSONL files with one JSON object per line, version-tagged, in ~/.xylitol/sessions/."
  s2,"entry-types","SessionManager MUST support entry types: message, compaction, branch_summary, model_change, thinking_level_change, custom."
  s3,crud,"SessionManager MUST support: create(id), append(id,entry), load(id), list(), exists(id)."
  s4,"version-migration","SessionManager MUST support version migration when reading older-format session files."
  s5,"session-tree","SessionManager MUST maintain parent/child session links: fork creates child, branch summaries reference parent entries."
  s6,"branch-summary","System MUST support generateBranchSummary(parentEntries) producing a CompactionEntry summarizing cut-point entries."
  s7,"file-operations","SessionManager MUST use atomic appends (append-only JSONL) with file locking for concurrent access safety."
  s8,"bdd-session",BDD tests under tests/features/session.feature MUST all pass.
scenarios[10]{req_id,id,given,when,then}:
  r1,happy,a snapshot exists,spawn is called with snapshot_id and new prompt,new agent instance starts with inherited context from snapshot
  r2,happy,conversation exceeds 75% of context window,compaction is triggered,older turns are summarized and replaced with compact system message
  s1,"create-load",a new session is created,entries are appended and session is loaded,all entries are returned in order
  s2,types,entries of each type are appended,session is loaded,each entry preserves its type and data
  s3,list,multiple sessions exist,list() is called,all session ids are returned
  s4,migration,a v2 session file exists,session is loaded,entries are migrated to v3 format correctly
  s5,fork,session A has 10 entries,fork at entry 5 creates session B,"B has entries 1-5 + branch_summary"
  s6,"branch-summary",5 entries before a cut point,generateBranchSummary is called,a CompactionEntry summarizing those 5 entries is produced
  s7,concurrent,two writers append to the same session,both writes complete,file has all entries without corruption
  s8,"bdd-pass",BDD runner invoked,"cargo test --test bdd",all session scenarios pass
```
