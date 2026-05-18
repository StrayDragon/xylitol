---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c65-add-skills-mcp"
---

```toon
kind: llman.sdd.spec
name: "skill-extension"
purpose: "TBD - created by archiving change c65-add-skills-mcp. Update purpose after archive."
requirements[2]{req_id,title,statement}:
  r1,"skill-yaml",System MUST load Skill definitions from YAML with name description system_prompt_addon and allowed_tools.
  r2,"mcp-client",System MUST connect to MCP servers via stdio or SSE transport and register their tools dynamically.
scenarios[2]{req_id,id,given,when,then}:
  r1,happy,a skill YAML file is configured,session starts with skill activated,skill system_prompt_addon is injected into agent context
  r2,happy,MCP server configured with stdio transport,agent starts,MCP tools are registered and callable as normal tools
```
