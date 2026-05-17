---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c20-add-tools"
---

```toon
kind: llman.sdd.spec
name: tool-system
purpose: TBD - created by archiving change c20-add-tools. Update purpose after archive.
requirements[3]{req_id,title,statement}:
  r1,tool-trait,System MUST define a Tool trait compatible with adk-core FunctionTool for all built-in tools.
  r2,seven-tools,"System MUST implement 7 built-in tools: read bash edit write grep find ls."
  r3,patch-apply,System MUST apply AI-generated patches using fudiff fuzzy matching with patch crate exact fallback.
scenarios[3]{req_id,id,given,when,then}:
  r1,happy,"",Tool trait is defined,it implements the adk-core FunctionTool interface
  r2,happy,a tool registry with all 7 tools,each tool is invoked with valid args,each returns a successful ToolResult
  r3,happy,an AI-generated unified diff with slight line offset,patch is applied via fudiff,fudiff successfully applies despite line offset
```
