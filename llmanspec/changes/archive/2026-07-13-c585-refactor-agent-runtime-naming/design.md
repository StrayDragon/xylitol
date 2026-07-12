# Design — c585-refactor-agent-runtime-naming

## Naming

```text
Driver ──持有──► AgentRuntime (原 ReActAgent)
                      │
                      └── inner: AgentCapabilities (原 session::Agent)
```

`domain::AgentContext` remains the LLM request snapshot — never reuse that name for the capability aggregate.

## Migration

1. Rename types + re-exports first; fix compile errors.
2. Update specs statements that MUST-name the old types.
3. Regenerate `tests/snapshots/agent_session_api_baseline*` if needed.
4. Update `.agents/skills/write-surface` symbol table.
