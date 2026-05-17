---
llman_spec_valid_scope:
  - src/agent/provider
llman_spec_valid_commands:
  - llman sdd validate c27-add-fake-provider --type spec --strict --no-interactive
  - cargo test -p xylitol -- dev_fake_provider
llman_spec_evidence:
  - cargo test dev_fake_provider passes with coverage of text, tool_call, and multi-step scenarios
---

```toon
kind: llman.sdd.delta
ops[3]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,provider-trait,"System MUST define a Provider trait with generate() that accepts a GenerateRequest and returns a GenerateResponse containing text deltas and/or tool calls.",null,null,null
  add_requirement,r2,fake-provider,"System MUST implement FakeProvider that implements Provider, returning predetermined responses configured via ScenarioStep sequences.",null,null,null
  add_requirement,r3,scenario-orchestration,"System MUST support orchestrating multi-step scenarios: text -> tool_call -> tool_result -> text, with configurable delays and error injection.",null,null,null
op_scenarios[3]{req_id,id,given,when,then}:
  r1,happy,"FakeProvider is configured with a text-only scenario","generate() is called","response contains the predetermined text with no tool calls"
  r2,happy,"FakeProvider is configured with a tool_call scenario including preset args","generate() is called","response contains a ToolCall with the configured name and args"
  r3,happy,"FakeProvider is configured with a multi-turn scenario (text, tool_call, tool_result, text)","generate() is called 4 times across the conversation turns","each response returns the expected step in order"
```
