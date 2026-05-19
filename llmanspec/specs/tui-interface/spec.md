---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - "Archived from change c80-add-tui"
---

```toon
kind: llman.sdd.spec
name: "tui-interface"
purpose: "TBD - created by archiving change c80-add-tui. Update purpose after archive."
requirements[10]{req_id,title,statement}:
  r1,"component-architecture","System MUST implement a Component trait with render(&mut self, f, area), is_dirty(), mark_clean(), and handle_event() methods for all major TUI components."
  r2,"event-driven-ui","System MUST consume AgentEvent stream via tokio::select! and update TUI in real-time with dirty-flag differential rendering."
  r3,"markdown-rendering","System MUST render markdown via pulldown-cmark (CommonMark spec) with syntect syntax highlighting for code blocks."
  r4,"multi-line-input","System SHALL provide multi-line text input via tui-textarea with Shift+Enter for newline, Enter to submit, and readline keybindings (Ctrl+K/U/W/A/E)."
  r5,"async-input-queue","System SHALL allow user to type and queue prompts while agent is running; queued messages auto-submit on agent completion."
  r6,"synchronized-rendering","System SHALL use crossterm BeginSynchronizedUpdate/EndSynchronizedUpdate for flicker-free rendering when supported."
  r7,"slash-commands","System SHALL support slash commands (/clear, /help, /quit) with tab completion in the input area."
  r8,"history-persistence",System SHALL persist input history to ~/.xylitol/history file and support up/down history navigation.
  r9,"overlay-stack","System SHALL implement a z-ordered OverlayStack for modals (help, selection) with stack-based input routing."
  r10,"status-bar","System SHALL display a status bar with model name, running/ready state, token usage, and session name."
scenarios[10]{req_id,id,given,when,then}:
  r1,happy,Component trait is defined,each major component implements it,is_dirty returns correct state after mutations
  r2,happy,agent emits TextDelta events via agent_tx channel,"App::handle_agent_event processes them",chat component marks dirty and content updates on next render
  r3,happy,"assistant message contains markdown with code blocks, lists, links","MarkdownRenderer::render() is called",output contains correctly styled ratatui Lines with syntax highlighting
  r4,happy,"user types multi-line text with Shift+Enter",text area grows vertically,"Enter submits full text, Ctrl+K deletes to end of line"
  r5,happy,agent is running and user types a message then presses Enter,input is accepted into a Vec<String> queue,"queue indicator appears in status bar; after agent completes, queued message auto-submits"
  r6,happy,"terminal emulator supports synchronized output (kitty, iTerm2)",render cycle begins,frame content writes are wrapped in BeginSynchronizedUpdate/EndSynchronizedUpdate
  r7,happy,user types /cl and presses Tab,slash command completion fires,/cl completes to /clear and pressing Enter runs the command
  r8,happy,user submits prompts in multiple sessions,history file is written and read,↑ key navigates through previously submitted prompts across sessions
  r9,happy,user presses ? to open help,OverlayStack.push(help) is called,keyboard events route to help overlay until Esc dismisses it
  r10,happy,agent starts execution,status_bar.set_running(true),"status bar shows RUNNING state, model name, and updates token count live"
```
