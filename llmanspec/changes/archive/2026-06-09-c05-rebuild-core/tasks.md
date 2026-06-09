# Tasks: c05-rebuild-core

## Phase 1: Tool System (基础设施 + 7 tools)

- [x] 1.1 TruncationResult: truncateHead, truncateTail, truncateLine
- [x] 1.2 Pluggable Operations traits: BashOperations, ReadOperations, WriteOperations, EditOperations, GrepOperations, FindOperations, LsOperations + default impls
- [x] 1.3 FileMutationQueue: serialize same-path writes/edits, parallel different-path
- [x] 1.4 Abort support: CancellationToken in XyTool::execute
- [x] 1.5 Path utils: resolve_to_cwd
- [x] 1.6 Edit tool: multi-edit + original-file matching + overlap/uniqueness/no-change checks + BOM/LF + fuzzy unicode + unified patch + display diff
- [x] 1.7 Grep tool: ripgrep + regex/glob/ignoreCase/literal/context/limit
- [x] 1.8 Find tool: fd + .gitignore + path relativization + limit
- [x] 1.9 Bash tool: process tree + abort + streaming + shell detection
- [x] 1.10 Read tool: truncation + offset bounds + remaining lines hint
- [x] 1.11 Write tool: mutation queue + abort + auto parent dirs
- [x] 1.12 Ls tool: sorted + `/` suffix + optional path + limit + entry limit hint
- [x] 1.13 ToolRegistry: builtins(), filtered(), get(), list()

## Phase 2: Session Persistence

- [x] 2.1 SessionEntry types: MessageEntry, CompactionEntry, BranchSummaryEntry, ModelChangeEntry, ThinkingLevelChangeEntry, CustomEntry
- [x] 2.2 SessionManager: create, append, load, list, JSONL file storage, version migration
- [x] 2.3 Session tree: parent/child linking, branch summaries
- [x] 2.4 BDD: session.feature scenarios pass

## Phase 3: Agent Session + Agent Loop

- [x] 3.1 AgentSession: prompt → model → tools → events loop, turn tracking, tool execution pipeline
- [x] 3.2 Event stream: turn_start/end, message_start/update/end, tool_execution_start/update/end
- [x] 3.3 Model switching: cycleForward/cycleBackward/select, model registry
- [x] 3.4 Thinking level toggle: low/medium/high, clamp to model capabilities
- [x] 3.5 BDD: agent.feature scenarios pass

## Phase 4: Compaction

- [x] 4.1 Context token estimation
- [x] 4.2 Compaction trigger: shouldCompact based on thresholds
- [x] 4.3 compact(): summarize old messages, keep recent turns
- [x] 4.4 BDD: compaction.feature scenarios pass

## Phase 5: CLI + Config + Hooks

- [x] 5.1 CLI args: parseArgs matching pi (model, session, prompt, print, rpc flags)
- [x] 5.2 YAML config: three-tier (global/project/user) with pi-equivalent settings
- [x] 5.3 Hook system upgrade: add after_provider_request, after_provider_response hook points for prefix-caching support
- [x] 5.4 Hook dispatcher: regex pattern matching for event types

## Phase 6: Integration + Cleanup

- [x] 6.1 Wire: CLI → config loader → agent session → agent loop → session persistence
- [x] 6.2 Remove: security-policy, repeat-guard, planning-orchestrator (code + specs)
- [x] 6.3 BDD: all .feature files pass `cargo test --test bdd -- --test-threads=1`
- [x] 6.4 QA: `cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo test --test bdd`
