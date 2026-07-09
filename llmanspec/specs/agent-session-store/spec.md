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
requirements[15]{req_id,title,statement}:
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
  s9,"session-fork","SessionManager MUST support fork(parent_id, child_id, at_entry_id) that copies parent entries up to the fork point into a new child session file and appends a branch_summary entry."
  s10,"branch-summary-impl","System MUST generate a branch summary when forking that describes the skipped entries: count of entries, entry types, last user message, and notable actions from tool calls."
  s11,"agent-fork","AgentSession MUST expose fork_session(at_entry_id) that creates a new child session via SessionManager::fork() and returns the child session id."
  s16,"session-cwd-validate","SessionManager MUST validate that the stored CWD exists and is accessible when loading a session from disk, returning an actionable error message when not found."
  s17,"bdd-session-v3",BDD tests for CWD validation MUST pass.
scenarios[19]{req_id,id,given,when,then}:
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
  s9,"basic-fork",session A has 10 entries with id running from e0 to e9,fork at entry e4,child session has 5 entries (e0..e4) plus a branch_summary entry
  s9,"full-fork",session A has 5 entries,fork at last entry e4,child has all 5 entries with no branch_summary
  s9,"parent-link",child session is created via fork,child session header is loaded,parent_session field matches parent id
  s10,summary,parent session has 3 user messages and 7 assistant messages with 5 tool calls,branch summary is generated,summary includes total entries count and tool call count
  s10,"empty-summary",no entries remain after fork point,branch summary is generated,summary is empty string or indicates nothing skipped
  s11,"agent-fork",agent session is active,AgentSession.fork_session(e4) is called,new child session id is returned and child session file exists on disk
  s16,"cwd-ok",session file has cwd /tmp and directory exists,load is called,entries are returned normally
  s16,"cwd-missing",session file has cwd /nonexistent,load is called,error references missing directory /nonexistent
  s17,"bdd-pass",BDD runner invoked,"cargo test --test bdd","all session-v3 scenarios pass"
```
