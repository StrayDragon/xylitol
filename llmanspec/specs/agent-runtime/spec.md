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
requirements[11]{req_id,title,statement}:
  r1,"execution-loop","System MUST implement a self-contained ReAct loop (XyRunner) that calls XyModel for generation and dispatches XyTool calls until completion or max_iterations."
  r2,"event-system","System MUST emit AgentEvent variants (TextDelta / ThinkingDelta / ToolCallStart / ToolCallEnd / StepComplete / Error) from XyRunner without any adk-core Event dependency."
  r3,"session-runtime",System MUST manage session history via XySession trait with InMemorySession as default backend.
  r4,"openai-provider","System MUST support OpenAI chat completions and streaming via async-openai with tool calling and base_url override."
  r5,"anthropic-provider",System MUST support Anthropic messages API and streaming via reqwest with tool_use and base_url override.
  r6,"content-types","System MUST define XyContent and XyPart types (Text / Thinking / FunctionCall / FunctionResponse) independent of adk-core."
  ar1,"react-loop","Agent runtime MUST execute a ReAct loop: send messages to model, collect response, execute tool calls, append results, repeat until stop or max iterations."
  ar2,streaming,"Agent runtime MUST stream model responses token-by-token via an event stream (TextDelta, ThinkingDelta, FunctionCall)."
  ar3,"tool-execution","Agent runtime MUST execute all tool calls from a single model response before the next model call, supporting both sequential and parallel execution modes."
  ar4,"iteration-limit","Agent runtime MUST enforce a configurable max_iterations limit, terminating with error when reached."
  ar5,"event-stream",Agent runtime MUST expose the loop as an async Stream<AgentEvent> for consumer layers (Print/TUI/RPC).
scenarios[12]{req_id,id,given,when,then}:
  r1,"react-loop",a mock XyModel returns text then tool call then final text,XyRunner executes,all events are emitted and loop terminates after tool response
  r1,"max-iterations",XyModel always returns tool calls,XyRunner reaches max_iterations,loop terminates with MaxIterationsReached error
  r2,"no-adk-events",agent is executing,events are emitted,"no adk_core::Event or adk_runner types appear in the event stream"
  r3,"session-history",a session has 3 turns of history,agent sends a new prompt,all previous turns are included in the XyModel request
  r4,"openai-streaming","an OpenAI-compatible endpoint is configured",agent sends a prompt,"streaming text and tool_call chunks are emitted as adk_core::LlmResponse"
  r5,"anthropic-streaming",an Anthropic endpoint is configured,agent sends a prompt,"streaming content_block_delta events are mapped to adk_core::LlmResponse"
  r6,"part-roundtrip","an XyPart::FunctionCall is created",it is serialized and deserialized,all fields (name / args / id) are preserved
  ar1,"text-only",a model that returns text with no tool calls,prompt is sent,text is streamed and loop terminates
  ar2,streaming,model streams chunks,each chunk is processed,"TextDelta events are emitted in real-time"
  ar3,"parallel-tools",model returns 3 tool calls with parallel execution mode,tools execute concurrently,all results are appended before next model call
  ar4,"max-iterations",max_iterations is set to 2,model keeps calling tools for 3 iterations,loop terminates with error after 2nd iteration
  ar5,consumer,an event stream consumer subscribes,loop runs,consumer receives all AgentEvent variants
```
