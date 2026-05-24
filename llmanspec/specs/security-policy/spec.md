---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c50-add-security"
---

```toon
kind: llman.sdd.spec
name: "security-policy"
purpose: "TBD - created by archiving change c50-add-security. Update purpose after archive."
requirements[9]{req_id,title,statement}:
  r1,"declarative-rules",System MUST enforce declarative security rules for bash/filesystem/network access before any tool execution.
  r2,"tighten-only","System MUST only allow three-tier config overrides to tighten rules never to relax them."
  r3,"unified-path-field-check",SecurityEngine MUST check both 'file_path' and 'path' argument fields when evaluating filesystem tool access.
  r4,"mcp-tool-security-policy","SecurityEngine MUST apply a dedicated policy branch for MCP tools (name prefix 'mcp:') with explicit server/tool allowlist and default-deny semantics."
  r5,"security-enabled-default",SecurityConfig.enabled MUST default to true in production builds.
  r6,"hook-timeout-kill","Hook executor MUST kill the child process on timeout and MUST support configurable fail-open or fail-closed behavior."
  r7,"network-domain-enforcement",SecurityEngine MUST enforce network.allowed_domains and network.blocked_domains for bash/curl and MCP SSE URLs.
  r8,"bash-timeout-cap",Bash tool MUST cap timeout to min(requested_timeout and config.security.bash.timeout_secs).
  r9,"bash-approval-required",TUI approval MUST cover bash tool execution when security is enabled.
scenarios[9]{req_id,id,given,when,then}:
  r1,happy,"bash tool called with rm -rf /","security policy has forbidden pattern for rm -rf",tool call is blocked and tool_call_blocked event emitted
  r2,happy,user config tries to allow a forbidden pattern,config is merged,the forbidden pattern remains blocked
  r3,"path-field-bypass","security enabled with forbidden_patterns=['/etc/**']",grep tool called with path='/etc/passwd',SecurityEngine returns Blocked
  r4,"mcp-default-deny",security enabled with no MCP allowlist,"agent calls mcp:server:tool",SecurityEngine returns Blocked with reason
  r5,"default-enabled",fresh install with no config override,SecurityEngine initializes,enabled field is true
  r6,"hook-kill-on-timeout",hook configured with timeout=100ms,hook script runs 'sleep 999',"child process is killed and action is Block (fail-closed default)"
  r7,"network-domain-block",blocked_domains contains 'evil.com',bash executes 'curl evil.com',SecurityEngine returns Blocked
  r8,"timeout-cap",config.security.bash.timeout_secs=30,LLM requests timeout=99999,actual timeout used is 30 seconds
  r9,"bash-needs-approval",security enabled in TUI mode,agent invokes bash tool,TUI shows approval overlay before execution
```
