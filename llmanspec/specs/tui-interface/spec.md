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
requirements[40]{req_id,title,statement}:
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
  r11,"async-input-queue","System SHALL queue user input when Enter is pressed during agent execution and auto-submit queued messages after current execution completes."
  r12,"selector-real-data","System SHALL populate session/model/theme selectors from AppConfig (agent.profiles) and SessionService (list_sessions), with fuzzy-filterable list rendering."
  r13,"editor-integration",System SHALL support Ctrl+G to open current input buffer in $EDITOR (or vim as fallback) and read back the content on editor exit.
  r14,"history-search",System SHALL support Ctrl+R interactive history search overlay with fuzzy matching.
  r15,"mouse-interaction",System SHALL process mouse scroll wheel for chat scrolling and mouse click for focus switching.
  r16,"focus-visual",System SHALL visually indicate the active focus area with highlighted border and title accent.
  r17,"pane-resize",System SHALL support Ctrl+Up/Down to resize the chat/tool output split.
  r18,"approval-wiring",System SHALL wire ApprovalOverlay to agent SecurityPolicy such that tool calls requiring approval show a modal with Allow/Deny/AllowOnce/DenyOnce options.
  r19,"diff-preview-wiring","System SHALL wire DiffPreviewOverlay to StepComplete events, collecting diffs from ReviewEngine and displaying them via Ctrl+R toggle."
  r20,"diff-type-reuse","System SHALL reuse diff_review::types::{DiffHunk, DiffLine, DiffLineKind} in diff_preview.rs, eliminating duplicate type definitions."
  r21,"tool-call-cards","System SHALL render tool calls as collapsible cards with status badge (running/success/failed), argument preview, and Enter to expand."
  r22,"thinking-blocks","System SHALL detect and render agent reasoning as collapsible dim-styled blocks, collapsed by default with Enter to expand."
  r23,"approval-with-diff","System SHALL show the diff of proposed tool changes inline within the approval overlay for edit-type tools."
  r24,"initial-paint","System MUST render the first TUI frame immediately after entering the alternate screen, before waiting for the first user input or tick event, so the user never sees a blank screen at startup."
  r25,"full-frame-render","System MUST render all visible TUI components on every `Terminal::draw` frame; dirty flags MUST only control whether a draw is scheduled, not whether individual components render within a frame."
  r26,"quit-cleanup","System MUST exit interactive mode on Ctrl+D (or /quit) without hanging and MUST restore the terminal (leave alternate screen, disable raw mode, show cursor)."
  r27,"codex-parity-keymap","System MUST provide a RuntimeKeymap abstraction (KeyBinding + context maps) and route TUI shortcuts through it, instead of matching raw KeyCode values directly."
  r28,"composer-tab-queue","System MUST implement Codex-style composer semantics where Tab queues the current draft while an agent task is running; when idle it behaves as submit except for bang-shell drafts."
  r29,"bang-shell-special-case","System MUST treat drafts starting with '!' as shell-mode drafts; when idle, Tab MUST NOT submit bang-shell drafts."
  r30,"composer-esc-cancel","System MUST implement Codex-style Esc handling in the composer: Esc dismisses any active popup; otherwise Esc cancels (clears) the current draft without quitting the app. When the composer is empty, Esc MUST prime backtrack rather than doing nothing."
  r31,"backtrack-esc-esc","When the composer is empty and the agent is idle, System MUST support a minimal backtrack flow where pressing Esc twice loads the last user message into the composer for editing."
  r32,"transcript-overlay",System MUST provide a transcript overlay toggled by Ctrl+T that shows the current conversation transcript and can be dismissed with Esc/q.
  r33,"copy-last-response","System MUST support copying the latest assistant response as Markdown via Ctrl+O (macOS pbcopy) with a cross-platform fallback."
  r34,"codex-style-layout","System MUST render the interactive TUI using a Codex-style transcript + bottom-pane layout (without separate boxed Chat/Tools/Input panels), while preserving xylitol backend semantics."
  r35,"codex-style-footer","System MUST render Codex-style footer/statusline hints (queue, shortcuts, running state) instead of the legacy StatusBar widget."
  r36,"codex-style-colors","System MUST follow Codex TUI style constraints (prefer default fg + dim, cyan for hints/selection, magenta for Codex identity; avoid heavy box borders) to match Codex look-and-feel."
  r37,"approval-no-race",TUI approval mechanism MUST ensure the approval channel is registered before the tool wrapper attempts to receive a decision.
  r38,"atomic-file-write",Write and edit tools MUST use atomic write (temp file + rename) to prevent partial writes on crash or concurrent access.
  r39,"hook-process-cleanup",Hook executor MUST terminate child processes on timeout via kill() and MUST wait for process exit before returning.
  r40,"mcp-lock-granularity",MCP client manager MUST NOT hold its mutex across await points to avoid serializing concurrent tool calls.
scenarios[40]{req_id,id,given,when,then}:
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
  r11,happy,agent is running and user types 'hello' then presses Enter,input is not disabled and Enter is accepted,"message is queued; status bar shows [Q: 1]; after agent completes, 'hello' is submitted automatically"
  r12,happy,user types /model and presses Enter,SelectorOverlay opens with model list from AppConfig,fuzzy search narrows the list; Enter selects a model; status bar updates
  r13,happy,user types text in input then presses Ctrl+G,$EDITOR (or vim) opens with the text,user saves and exits; text is read back into the input area
  r14,happy,user presses Ctrl+R during input,history search overlay opens,fuzzy matching against history file; Enter loads selected entry into input
  r15,happy,mouse scroll wheel is used over the chat area,"App handles MouseEvent::ScrollDown",chat scrolls down by N lines
  r16,happy,user presses Shift+Tab to cycle focus,"App::cycle_focus() changes focus field",focused component gets a highlighted border (cyan) vs dim border for unfocused
  r17,happy,tool panel is visible and user presses Ctrl+Up,"tool panel Constraint::Length increases by 1",layout adjusts; chat area shrinks accordingly
  r18,happy,agent calls a tool that SecurityPolicy flags as needing approval,ToolCallStart triggers ApprovalOverlay.prompt(),overlay shows tool name and args; user selects Allow; tool executes
  r19,happy,agent completes a step with file edits,StepComplete event fires with diffs,DiffPreviewOverlay.set_diff() is called; Ctrl+R shows the diff
  r20,happy,diff_preview.rs renders a diff,"it imports DiffHunk from diff_review::types",no duplicate DiffLineKind enum exists in the codebase
  r21,happy,agent starts a tool call during streaming,chat receives ToolCallStart event,"a collapsible card renders with a spinner; on ToolCallEnd, spinner changes to checkmark; Enter toggles details"
  r22,happy,assistant message contains thinking content delimited by markers,chat component detects the pattern,content renders as a collapsed dim block; Enter expands to full text
  r23,happy,a write/edit tool requires approval,ApprovalOverlay shows with diff preview,the diff of the file change is visible within the overlay before user decides
  r24,happy,TUI starts and enters alternate screen,initial render is performed,input box and status bar are visible without requiring key press
  r25,happy,event loop runs and no new events occur,a redraw happens (tick/agent update/etc),previously rendered UI remains visible (no blank/black frame)
  r26,happy,user is in interactive mode,Ctrl+D is pressed,process exits cleanly and returns to shell prompt without requiring Ctrl+C
  r27,happy,"",user presses a configured shortcut key,the corresponding action is dispatched via RuntimeKeymap matching
  r28,happy,"",agent is running and user presses Tab,draft is queued (not submitted) and status bar queue length increments
  r29,happy,"",agent is idle and composer starts with '!',pressing Tab does not submit or clear the draft
  r30,happy,"","composer has non-empty draft and user presses Esc",draft is cleared (and popup dismissed if active)
  r31,happy,"",composer is empty and user presses Esc twice,last user message is loaded into composer
  r32,happy,"",user presses Ctrl+T,transcript overlay opens and can be dismissed with Esc
  r33,happy,"",user presses Ctrl+O,latest assistant response is copied (or written to a temp file on fallback)
  r34,happy,"",TUI starts,"the screen shows a transcript area and a bottom composer/footer, without boxed Chat/Tools/Input headers"
  r35,happy,"",agent task is running and the composer is empty,"footer shows running hint and Tab queue hint in a single-line Codex-style footer"
  r36,happy,"",rendering transcript and composer,"UI uses limited Codex palette (default/dim/cyan/magenta/green/red) and does not draw Borders::ALL boxes for major panels"
  r37,"no-spurious-reject",TUI mode with approval enabled,agent calls a tool requiring approval immediately after stream yields,tool waits for user decision without 'missing approval prompt' error
  r38,"crash-safe-write",write tool is called,"process crashes mid-write",original file remains intact (no partial content)
  r39,"hook-zombie-prevention",hook script exceeds timeout,timeout fires,child process is killed and no zombie remains
  r40,"concurrent-mcp",two MCP tools are called concurrently,both execute,neither blocks waiting for the other's completion
```
