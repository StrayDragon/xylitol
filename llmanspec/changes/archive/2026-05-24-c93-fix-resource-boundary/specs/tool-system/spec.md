---
llman_spec_valid_scope:
  - src/agent/tools
  - src/infra/session/storage.rs
  - src/agent/repeat.rs
llman_spec_valid_commands:
  - llman sdd validate c93-fix-resource-boundary --type spec --strict --no-interactive
llman_spec_evidence:
  - read tool rejects files larger than MAX_FILE_SIZE
  - bash tool kills process on output overflow
  - zstd decode respects max output size
---

```toon
kind: llman.sdd.delta
ops[5]{op,req_id,title,statement,from,to,name}:
  add_requirement,r4,read-file-size-limit,"Read tool MUST reject files exceeding MAX_FILE_SIZE (default 10MB) with a clear error message.",null,null,null
  add_requirement,r5,bash-streaming-output,"Bash tool MUST stream stdout/stderr with a byte cap and MUST kill the child process when the cap is exceeded.",null,null,null
  add_requirement,r6,integer-cast-validation,"Tools MUST validate numeric arguments (timeout and max_results) are within positive safe ranges before casting.",null,null,null
  add_requirement,r7,find-no-absolute-escape,"Find tool MUST reject absolute patterns or MUST canonicalize results to verify they remain within the root directory.",null,null,null
  add_requirement,r8,safe-utf8-truncation,"Output truncation MUST use character-boundary-aware slicing to prevent UTF-8 panics.",null,null,null
op_scenarios[5]{req_id,id,given,when,then}:
  r4,large-file,"a 50MB file exists","read tool is called on it without offset/limit","tool returns error indicating file too large"
  r5,output-overflow,"bash runs 'yes' command","stdout exceeds 1MB cap","child process is killed and truncated output is returned"
  r6,negative-timeout,"LLM passes timeout=-1","bash tool validates the argument","tool rejects with invalid timeout error"
  r7,absolute-pattern,"find called with pattern='/etc/**'","security is enabled","tool returns error or filters results to root"
  r8,multibyte-truncation,"bash output ends with incomplete UTF-8 sequence at cap boundary","truncate_output is called","output is safely truncated at character boundary without panic"
```
