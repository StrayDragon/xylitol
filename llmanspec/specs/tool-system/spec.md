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
name: "tool-system"
purpose: "TBD - created by archiving change c20-add-tools. Update purpose after archive."
requirements[11]{req_id,title,statement}:
  r1,"tool-trait","System MUST define a XyTool trait as the primary tool interface; adk-core Tool compatibility MUST be provided via adapter only."
  r2,"seven-tools","System MUST implement 7 built-in tools: read bash edit write grep find ls."
  r3,"patch-apply","System MUST apply AI-generated patches using fudiff fuzzy matching with patch crate exact fallback."
  r4,"read-file-size-limit",Read tool MUST reject files exceeding MAX_FILE_SIZE (default 10MB) with a clear error message.
  r5,"bash-streaming-output",Bash tool MUST stream stdout/stderr with a byte cap and MUST kill the child process when the cap is exceeded.
  r6,"integer-cast-validation",Tools MUST validate numeric arguments (timeout and max_results) are within positive safe ranges before casting.
  r7,"find-no-absolute-escape",Find tool MUST reject absolute patterns or MUST canonicalize results to verify they remain within the root directory.
  r8,"safe-utf8-truncation","Output truncation MUST use character-boundary-aware slicing to prevent UTF-8 panics."
  r9,"tool-args-helper",Tool system MUST provide a shared argument extraction helper that eliminates repetitive JSON field parsing across tool implementations.
  r10,"no-global-dead-code-allow","Crate root MUST NOT use global #![allow(dead_code)]; dead code suppression MUST be scoped to individual items with justification."
  r11,"tool-error-types","System MUST define XyToolError with structured error categories (invalid_args / execution_failed / permission_denied / timeout) independent of adk-core."
scenarios[11]{req_id,id,given,when,then}:
  r1,"xy-tool-impl","all 7 built-in tools implement XyTool",each tool is invoked,each returns Result<String> without any adk_core types in the call chain
  r2,happy,a tool registry with all 7 tools,each tool is invoked with valid args,each returns a successful ToolResult
  r3,happy,"an AI-generated unified diff with slight line offset",patch is applied via fudiff,fudiff successfully applies despite line offset
  r4,"large-file",a 50MB file exists,read tool is called on it without offset/limit,tool returns error indicating file too large
  r5,"output-overflow",bash runs 'yes' command,stdout exceeds 1MB cap,child process is killed and truncated output is returned
  r6,"negative-timeout","LLM passes timeout=-1",bash tool validates the argument,tool rejects with invalid timeout error
  r7,"absolute-pattern",find called with pattern='/etc/**',security is enabled,tool returns error or filters results to root
  r8,"multibyte-truncation","bash output ends with incomplete UTF-8 sequence at cap boundary",truncate_output is called,output is safely truncated at character boundary without panic
  r9,happy,a tool needs a required string argument,tool calls require_str(args and 'file_path'),returns Ok(value) or Err(AdkError) with consistent error code
  r10,happy,crate compiles with dead_code lint enabled,cargo clippy runs,no dead_code warnings in production code paths
  r11,"error-mapping","a tool returns XyToolError::InvalidArgs",error is propagated to agent loop,error category is preserved and displayed to user
```
