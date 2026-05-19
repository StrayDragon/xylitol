```toon
kind: llman.sdd.delta
ops[7]{op,req_id,title,statement,from,to,name}:
  add_requirement,r11,async-input-queue,"System SHALL queue user input when Enter is pressed during agent execution and auto-submit queued messages after current execution completes.",null,null,null
  add_requirement,r12,selector-real-data,"System SHALL populate session/model/theme selectors from AppConfig (agent.profiles) and SessionService (list_sessions), with fuzzy-filterable list rendering.",null,null,null
  add_requirement,r13,editor-integration,"System SHALL support Ctrl+G to open current input buffer in $EDITOR (or vim as fallback) and read back the content on editor exit.",null,null,null
  add_requirement,r14,history-search,"System SHALL support Ctrl+R interactive history search overlay with fuzzy matching.",null,null,null
  add_requirement,r15,mouse-interaction,"System SHALL process mouse scroll wheel for chat scrolling and mouse click for focus switching.",null,null,null
  add_requirement,r16,focus-visual,"System SHALL visually indicate the active focus area with highlighted border and title accent.",null,null,null
  add_requirement,r17,pane-resize,"System SHALL support Ctrl+Up/Down to resize the chat/tool output split.",null,null,null
op_scenarios[7]{req_id,id,given,when,then}:
  r11,happy,"agent is running and user types 'hello' then presses Enter","input is not disabled and Enter is accepted","message is queued; status bar shows [Q: 1]; after agent completes, 'hello' is submitted automatically"
  r12,happy,"user types /model and presses Enter","SelectorOverlay opens with model list from AppConfig","fuzzy search narrows the list; Enter selects a model; status bar updates"
  r13,happy,"user types text in input then presses Ctrl+G","$EDITOR (or vim) opens with the text","user saves and exits; text is read back into the input area"
  r14,happy,"user presses Ctrl+R during input","history search overlay opens","fuzzy matching against history file; Enter loads selected entry into input"
  r15,happy,"mouse scroll wheel is used over the chat area","App handles MouseEvent::ScrollDown","chat scrolls down by N lines"
  r16,happy,"user presses Tab to cycle focus","App::cycle_focus() changes focus field","focused component gets a highlighted border (cyan) vs dim border for unfocused"
  r17,happy,"tool panel is visible and user presses Ctrl+Up","tool panel Constraint::Length increases by 1","layout adjusts; chat area shrinks accordingly"
```
