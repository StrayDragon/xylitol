# Design: c1750 compaction-aware session context

## pi `buildContextEntries`

```text
path = getBranch(leaf)
compaction = last compaction on path
if none: return path
out = [compaction]
  + path[firstKept .. compaction)   # found by firstKeptEntryId
  + path[compaction+1 ..]
```

## xylitol placement

- Pure fn on `&[SessionEntry]` in `infra/session/manager.rs` (or `protocol/session.rs`)
- Call sites: `build_session_context`, `build_session_context_v2`, `AgentCapabilities::load_conversation_history`
- ReAct overflow retry: replace `history` with reloaded leaf context (project already drops error assistants)

## Error unwrap

`OpenAI Responses: … content:{"error":{"code":400,"message":"…","type":"…"}}`
→ surface `message` (+ optional type) so overflow regex `exceeds the available context size` matches.
