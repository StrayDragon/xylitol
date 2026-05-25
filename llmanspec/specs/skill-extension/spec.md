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
  r1,"mcp-adapter","McpToolAdapter MUST bridge rmcp tool results to XyTool interface (not adk_core::Tool)."
  r2,"mcp-client",System MUST connect to MCP servers via stdio or SSE transport and register their tools dynamically.
scenarios[2]{req_id,id,given,when,then}:
  r1,"mcp-as-xytool",an MCP server exposes a tool,McpToolAdapter wraps it as XyTool,tool can be registered in ToolRegistry and executed by agent loop
  r2,happy,MCP server configured with stdio transport,agent starts,MCP tools are registered and callable as normal tools
```
