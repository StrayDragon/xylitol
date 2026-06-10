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
requirements[12]{req_id,title,statement}:
  c1,"token-estimation",System MUST estimate context window usage from current messages using a configurable token estimator.
  c2,"should-compact",System MUST trigger compaction when estimated token usage exceeds a configurable threshold percentage of the context window.
  c3,"compact-summarize",Compaction MUST summarize older messages into a CompactionEntry while preserving the N most recent turns.
  c4,"compact-session",Compaction MUST persist the CompactionEntry to the session file and update the session tree.
  c5,"branch-summary","When navigating to a session tree branch, System MUST generate a branch summary entry bridging the context gap."
  c6,"bdd-compaction",BDD tests under tests/features/compaction.feature MUST all pass.
  c7,"llm-summarize","Compaction MUST call a configured LLM to generate a structured summary using format: ## Goal / ## Constraints & Preferences / ## Progress (Done, In Progress, Blocked) / ## Key Decisions / ## Next Steps / ## Critical Context."
  c8,"cut-point","System MUST find a cut point in session entries that preserves approximately keepRecent tokens of recent context, walking backwards from newest entries and cutting at valid entry types (user/assistant/custom/branch_summary, never tool results)."
  c9,"iterative-summary","When a previous CompactionEntry exists, System MUST use an update prompt that preserves existing structured information and merges new progress, rather than starting from scratch."
  c10,"file-tracking","Compaction summary MUST append <read-files> and <modified-files> XML tags listing files referenced in tool calls within the summarized messages."
  c11,"compact-entry","Compaction MUST produce a CompactionEntry containing summary string, firstKeptEntryId referencing the first retained entry after the cut point, tokensBefore count, and details object with readFiles and modifiedFiles lists."
  c12,"agent-integration","AgentSession MUST expose compact_current_session() that: estimates context usage, finds cut point, calls LLM summarization, writes CompactionEntry, and reloads session state."
scenarios[12]{req_id,id,given,when,then}:
  c1,estimate,messages contain 1000 tokens of text,estimateTokens is called,result is approximately 1000 tokens
  c2,trigger,tokens exceed 80% of context window,shouldCompact is called,returns true
  c3,summarize,session has 50 turns,compact is called,first 40 turns are summarized into one CompactionEntry
  c4,persist,compaction completes,session is loaded,CompactionEntry is present with summary and cut point
  c5,branch,user navigates to earlier branch point,branch summary is generated,summary entry bridges the context gap
  c6,"bdd-pass",BDD runner invoked,"cargo test --test bdd",all compaction scenarios pass
  c7,summarize,session has 30 user+assistant turns with file edits,generate_summary is called,response contains Goal Progress Next Steps sections with specific file paths
  c8,"find-cut",session has 50 entries totaling 80000 tokens with keepRecent=20000,find_cut_point is called,the cut index preserves roughly the last 20000 tokens of context
  c9,iterative,"a previous CompactionEntry exists with summary, new messages are accumulated",generate_summary is called with previousSummary,result preserves previous Done items and adds new ones
  c10,files,"messages contain tool calls: read a.txt, write b.rs, edit c.py",compact_session is called,"CompactionEntry summary ends with <read-files>a.txt</read-files> and <modified-files>b.rs c.py</modified-files>"
  c11,entry,compact_session completes and session is loaded,CompactionEntry is present,"summary non-empty, firstKeptEntryId valid, tokensBefore positive, details contains file lists"
  c12,agent,agent session has messages exceeding threshold,compact_current_session is called,"CompactionEntry written to session, session state reloaded"
```
