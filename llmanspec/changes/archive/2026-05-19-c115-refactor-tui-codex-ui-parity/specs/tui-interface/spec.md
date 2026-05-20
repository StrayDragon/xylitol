```toon
kind: llman.sdd.delta
ops[3]{op,req_id,title,statement,from,to,name}:
  add_requirement,r34,"codex-style-layout","System MUST render the interactive TUI using a Codex-style transcript + bottom-pane layout (without separate boxed Chat/Tools/Input panels), while preserving xylitol backend semantics.",null,null,null
  add_requirement,r35,"codex-style-footer","System MUST render Codex-style footer/statusline hints (queue, shortcuts, running state) instead of the legacy StatusBar widget.",null,null,null
  add_requirement,r36,"codex-style-colors","System MUST follow Codex TUI style constraints (prefer default fg + dim, cyan for hints/selection, magenta for Codex identity; avoid heavy box borders) to match Codex look-and-feel.",null,null,null
op_scenarios[3]{req_id,id,given,when,then}:
  r34,happy,"",TUI starts,"the screen shows a transcript area and a bottom composer/footer, without boxed Chat/Tools/Input headers"
  r35,happy,"",agent task is running and the composer is empty,"footer shows running hint and Tab queue hint in a single-line Codex-style footer"
  r36,happy,"",rendering transcript and composer,"UI uses limited Codex palette (default/dim/cyan/magenta/green/red) and does not draw Borders::ALL boxes for major panels"
```
