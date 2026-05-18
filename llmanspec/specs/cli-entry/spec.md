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
name: cli-entry
purpose: CLI argument parsing and mode dispatch for xylitol.
requirements[4]{req_id,title,statement}:
  r1,clap-args,System MUST parse CLI arguments via clap derive including prompt/config/project/model/list-models/yolo options.
  r2,auto-mode-detect,System MUST auto-detect mode: --acp flag for ACP, prompt present for print/stdio, no prompt for interactive/TUI.
  r3,list-models,System MUST provide --list-models flag that prints available models from config and exits.
  r4,fake-model-cli,System MUST support --model __fake__ to activate fake provider when dev-fake-provider feature is enabled.
scenarios[4]{req_id,id,given,when,then}:
  r1,happy,"",xylitol --config ./test.yaml "do something" is run,args are parsed with config path set and prompt present
  r2,happy,prompt is provided,CLI starts,print/stdio mode is auto-detected and used
  r2,no-prompt,no prompt is provided and ui-tui feature is enabled,CLI starts,interactive/TUI mode is auto-detected
  r2,acp-flag,--acp flag is provided,CLI starts,ACP mode is activated
  r3,happy,config has models defined,xylitol --list-models is run,model table is printed with aliases and providers
  r4,happy,dev-fake-provider feature is enabled,xylitol --model __fake__ "test" is run,fake provider is used without API keys
```
