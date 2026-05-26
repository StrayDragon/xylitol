---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c25-add-agent-loop"
---

```toon
kind: llman.sdd.spec
name: "agent-runtime"
purpose: "TBD - created by archiving change c25-add-agent-loop. Update purpose after archive."
requirements[6]{req_id,title,statement}:
  r1,"execution-loop","System MUST implement a self-contained ReAct loop (XyRunner) that calls XyModel for generation and dispatches XyTool calls until completion or max_iterations."
  r2,"event-system","System MUST emit AgentEvent variants (TextDelta / ThinkingDelta / ToolCallStart / ToolCallEnd / StepComplete / Error) from XyRunner without any adk-core Event dependency."
  r3,"session-runtime",System MUST manage session history via XySession trait with InMemorySession as default backend.
  r4,"openai-provider","System MUST support OpenAI chat completions and streaming via async-openai with tool calling and base_url override."
  r5,"anthropic-provider",System MUST support Anthropic messages API and streaming via reqwest with tool_use and base_url override.
  r6,"content-types","System MUST define XyContent and XyPart types (Text / Thinking / FunctionCall / FunctionResponse) independent of adk-core."
scenarios[7]{req_id,id,given,when,then}:
  r1,"react-loop",a mock XyModel returns text then tool call then final text,XyRunner executes,all events are emitted and loop terminates after tool response
  r1,"max-iterations",XyModel always returns tool calls,XyRunner reaches max_iterations,loop terminates with MaxIterationsReached error
  r2,"no-adk-events",agent is executing,events are emitted,"no adk_core::Event or adk_runner types appear in the event stream"
  r3,"session-history",a session has 3 turns of history,agent sends a new prompt,all previous turns are included in the XyModel request
  r4,"openai-streaming","an OpenAI-compatible endpoint is configured",agent sends a prompt,"streaming text and tool_call chunks are emitted as adk_core::LlmResponse"
  r5,"anthropic-streaming",an Anthropic endpoint is configured,agent sends a prompt,"streaming content_block_delta events are mapped to adk_core::LlmResponse"
  r6,"part-roundtrip","an XyPart::FunctionCall is created",it is serialized and deserialized,all fields (name / args / id) are preserved
```
