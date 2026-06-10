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
name: "agent-session"
purpose: "TBD - created by archiving change c05-rebuild-core. Update purpose after archive."
requirements[10]{req_id,title,statement}:
  a1,"agent-session","System MUST provide AgentSession encapsulating: prompt→model→tools→loop with full event stream."
  a2,"turn-events","AgentSession MUST emit: turn_start, message_start, message_update (streaming), message_end events per turn."
  a3,"tool-events","AgentSession MUST emit: tool_execution_start, tool_execution_update (streaming partials), tool_execution_end events per tool call."
  a4,"model-switching","AgentSession MUST support cycleForward, cycleBackward, and select(model) for runtime model switching."
  a5,"thinking-level","AgentSession MUST support thinking level toggle (low/medium/high), clamped to model capabilities."
  a6,abort,AgentSession MUST support abort() to cancel current agent operation.
  a7,"context-usage",AgentSession MUST expose getContextUsage() returning estimated token count and context window percentage.
  a8,"session-persistence","AgentSession MUST auto-persist messages to SessionManager after each turn and after tool execution."
  a9,"prompt-construction","AgentSession MUST construct the full messages array: system prompt + context files + history + user prompt, prepended per turn."
  a10,"bdd-agent",BDD tests under tests/features/agent.feature MUST all pass.
scenarios[10]{req_id,id,given,when,then}:
  a1,run,a model and tools are configured,agent is started with a prompt,text response is streamed via events
  a2,"turn-events","agent processes a tool-calling turn",turn starts,"events are emitted in order: turn_start message_start message_update* message_end turn_end"
  a3,"tool-stream",bash tool streams output,tool_execution_start fires,tool_execution_update fires multiple times then tool_execution_end
  a4,"switch-model",agent is running,cycleForward is called,next available model becomes active
  a5,"thinking-toggle",model supports thinking,thinking level is changed,new level is clamped to model capabilities
  a6,abort,agent is streaming a response,abort() is called,agent loop terminates and returns abort error
  a7,"context-usage",a model with 200k window is active,messages consume 50k tokens,getContextUsage() returns 25% and 50000 tokens
  a8,"auto-save",a turn completes,session is loaded from disk,the turn's messages are persisted
  a9,"prompt-build",system prompt is configured with context files,agent starts a turn,messages array has system prompt then history then user message
  a10,"bdd-pass",BDD runner invoked,"cargo test --test bdd",all agent scenarios pass
```
