---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c15-add-cli"
  - "Updated by change c03-update-cli-mode-dispatch"
---

```toon
kind: llman.sdd.spec
name: "cli-entry"
purpose: CLI argument parsing and mode dispatch for xylitol.
requirements[9]{req_id,title,statement}:
  r1,"clap-args","System MUST parse CLI arguments via clap derive including prompt/config/project/model/list-models/yolo options."
  r2,"auto-mode-detect","System MUST auto-detect mode: --acp flag for ACP, prompt present for print/stdio, no prompt requires explicit prompt."
  r3,"list-models","System MUST provide --list-models flag that prints available models from config and exits."
  r4,"fake-model-cli","System MUST support --model __fake__ to activate fake provider when dev-fake-provider feature is enabled."
  cli1,"parse-args","CLI MUST parse arguments matching pi: --model, --session, --print, --rpc, --prompt, positional prompt, --resume."
  cli2,"initial-message","CLI MUST support initial message from: positional argument, --prompt flag, or piped stdin."
  cli3,"session-picker","When no --session is given and sessions exist, CLI MUST offer a selector to create new or resume existing sessions."
  cli4,"project-trust","On first run in a directory, CLI MUST prompt for project trust confirmation."
  cli5,"run-modes","CLI MUST route to the appropriate mode handler: interactive (default), print (--print), or RPC (--rpc)."
scenarios[11]{req_id,id,given,when,then}:
  r1,happy,"","xylitol --config ./test.yaml \"do something\" is run",args are parsed with config path set and prompt present
  r2,happy,prompt is provided,CLI starts,"print/stdio mode is auto-detected and used"
  r2,"no-prompt",no prompt is provided,CLI starts,error is returned suggesting to provide a prompt
  r2,"acp-flag","--acp flag is provided",CLI starts,ACP mode is activated
  r3,happy,config has models defined,"xylitol --list-models is run",model table is printed with aliases and providers
  r4,happy,"dev-fake-provider feature is enabled","xylitol --model __fake__ \"test\" is run",fake provider is used without API keys
  cli1,positional,./xylitol 'fix the bug',args are parsed,prompt contains 'fix the bug'
  cli2,stdin,echo 'review this' | ./xylitol,args are parsed,prompt contains 'review this'
  cli3,picker,"sessions exist and no --session given",CLI starts,"a session picker is shown (print-mode skips this)"
  cli4,trust,first run in /home/user/project,CLI starts,a project trust prompt is shown
  cli5,print,"./xylitol --print 'fix'",CLI runs,output goes to stdout
```
