```toon
kind: llman.sdd.delta
ops[3]{op,req_id,title,statement,from,to,name}:
  add_requirement,r24,initial-paint,"System MUST render the first TUI frame immediately after entering the alternate screen, before waiting for the first user input or tick event, so the user never sees a blank screen at startup.",null,null,null
  add_requirement,r25,full-frame-render,"System MUST render all visible TUI components on every `Terminal::draw` frame; dirty flags MUST only control whether a draw is scheduled, not whether individual components render within a frame.",null,null,null
  add_requirement,r26,quit-cleanup,"System MUST exit interactive mode on Ctrl+D (or /quit) without hanging and MUST restore the terminal (leave alternate screen, disable raw mode, show cursor).",null,null,null
op_scenarios[3]{req_id,id,given,when,then}:
  r24,happy,"TUI starts and enters alternate screen","initial render is performed","input box and status bar are visible without requiring key press"
  r25,happy,"event loop runs and no new events occur","a redraw happens (tick/agent update/etc)","previously rendered UI remains visible (no blank/black frame)"
  r26,happy,"user is in interactive mode","Ctrl+D is pressed","process exits cleanly and returns to shell prompt without requiring Ctrl+C"
```
