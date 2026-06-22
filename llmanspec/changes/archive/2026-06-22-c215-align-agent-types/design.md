---
depends_on:
  - c170-refactor-agent-message-types
  - c175-refactor-event-system
  - c180-rebuild-agent-session
  - c185-upgrade-agent-loop
  - c190-extend-session-manager
  - c195-wire-compaction-lifecycle
  - c200-integrate-skills
  - c205-expand-provider-layer
  - c210-enhance-tool-system
---

# c215-align-agent-types: Design

## Overall Strategy

Single-shot migration: all type changes, history migration, and cleanup happen in one pass. No backwards-compatibility shims — every call site is updated immediately.

## Type Hierarchy

```
AgentMessage (enum, 7 variants)
├── UserMessage { content, timestamp }
├── AssistantMessage { content, stop_reason, usage, api, provider, model,
│     response_id, error_message, timestamp, diagnostics }
├── ToolResultMessage { tool_use_id, tool_name, content, details, is_error, timestamp }
├── BashExecutionMessage { command, output, exit_code, cancelled, truncated,
│     exclude_from_context }
├── CustomMessage { custom_type, content, display, details }
├── CompactionSummaryMessage { summary, tokens_before, tokens_after,
│     read_files, modified_files }
└── BranchSummaryMessage { summary, from_id }

AgentPart (enum, 5 variants)
├── Text(String)
├── Image(ImageContent)
├── Thinking(String, redacted: bool, signature: Option<String>)
├── ToolCall { id, name, arguments }
└── ToolResult { tool_use_id, content, is_error }

Usage → UsageCost { input, output, cache_read, cache_write, total: f64 }
     → cache_write_1h: u64

StopReason: Stop | MaxTokens | Error | Aborted | ToolUse
```

## Migration Path

Phase 1: Expand AgentMessage variants (add missing fields, keep from_xy_content working)
Phase 2: Rewrite loop.rs history type → Vec<AgentMessage>, update all internal users
Phase 3: Add lifecycle events
Phase 4: Expand ModelMeta
Phase 5: Delete old types

Each phase compiles and passes tests before the next begins.

## AgentState / AgentContext

Add to `message.rs` as lightweight structs. AgentSession holds an `AgentState` and exposes it. AgentLoop builds `AgentContext` before each LLM call.

## ModelManifest

The JSON format already has cost/maxTokens fields from the pi models.generated.ts format. We read them into ModelMeta fields.
