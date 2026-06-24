//! TUI interactive mode — Codex-style single-column transcript + composer.
//!
//! Uses the pi-style engine pathway: differential ANSI rendering direct to stdout
//! (no ratatui widgets, no alternate screen). Native text selection works.
//!
//! Event loop: select! over three sources:
//! - agent_rx: AgentEvent stream from AgentLoop
//! - input_rx: crossterm keyboard/mouse events
//! - frame_timer: throttle redraw to ~60fps

pub(crate) mod engine;
pub(crate) mod input;
pub(crate) mod state;

use std::io::{self, Write};
use std::time::Duration;

use crate::agent::r#loop::{AgentEvent, AgentEventStream, AgentLoop};
use crate::interface::tui::engine::ansi;
use crate::interface::tui::engine::renderer::TuiRenderer;
use crate::interface::tui::input::action::Action;
use crate::interface::tui::input::decode::DecodedInput;
use crate::interface::tui::input::keymap::{AgentState, resolve};
use crate::interface::tui::state::App;
use crossterm::event::{EventStream, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use futures::{FutureExt, StreamExt};

/// Run the TUI using the pi-style engine (differential ANSI rendering).
///
/// Key features:
/// - Uses `TuiRenderer` instead of `Terminal<CrosstermBackend>`
/// - Does NOT use alternate screen (native text selection works!)
/// - Renders directly to stdout with diff-based updates
/// - Does NOT call `agent_loop.run()` until the user submits their first
///   message (see `no-premature-agent`).
pub(crate) async fn run_tui_engine(
    agent_loop: &mut AgentLoop,
    session_id: &str,
    model_name: &str,
) -> Result<(), String> {
    // ── Terminal setup ──────────────────────────────────────────────
    setup_engine().map_err(|e| format!("terminal setup: {e}"))?;
    let mut renderer = TuiRenderer::new(io::stdout());

    // ── Agent stream ────────────────────────────────────────────────
    // `None` until the user submits their first message (`no-premature-agent`).
    // Once a stream is exhausted it is reset to `None`, which makes the agent
    // branch of `select!` pending — the same effect as c82's `agent_done`
    // flag, preventing starvation when `next()` would otherwise keep yielding
    // `Poll::Ready(None)`.
    let mut agent_stream: Option<AgentEventStream> = None;
    let mut pending_prompt: Option<String> = None;

    // ── Input channel ───────────────────────────────────────────────
    let mut input_stream = EventStream::new();

    // ── App state ───────────────────────────────────────────────────
    let mut app = App::new();

    // ── Frame timer (60fps = 16ms) ──────────────────────────────────
    let mut frame_interval = tokio::time::interval(Duration::from_millis(16));
    let mut dirty = true;

    // Get terminal size
    let (mut width, mut height) = terminal_size();

    // Model name for status bar
    let model_name_owned = model_name.to_string();

    // ── Main event loop ─────────────────────────────────────────────
    loop {
        // Lazily (re)start the agent when a user-submitted prompt is pending.
        // On startup `pending_prompt` is `None`, so no `agent_loop.run()` call
        // is made until the first message is submitted.
        if let Some(next_prompt) = pending_prompt.take() {
            agent_stream = Some(agent_loop.run(&next_prompt, session_id).await);
            dirty = true;
        }

        // When there is no active agent stream, use `pending()` so `select!`
        // does not spin on an always-ready `Poll::Ready(None)` future, which
        // would starve the input and frame timer branches (c82 Bug 1).
        let agent_fut = match agent_stream.as_mut() {
            Some(s) => s.next().boxed(),
            None => futures::future::pending::<Option<AgentEvent>>().boxed(),
        };

        tokio::select! {
            // Branch 1: Agent events
            event = agent_fut => {
                match event {
                    Some(agent_event) => {
                        let was_running = app.transcript.is_running();
                        app.transcript.apply(agent_event.clone());
                        dirty = true;

                        // Auto-dequeue: when agent finishes and there's queued input
                        if was_running && !app.transcript.is_running() {
                            if let Some(next) = app.composer.dequeue() {
                                pending_prompt = Some(next);
                            }
                        }
                    }
                    None => {
                        // Stream exhausted — drop it so the agent branch
                        // becomes pending and does not starve the loop.
                        agent_stream = None;
                        dirty = true;
                    }
                }
            }

            // Branch 2: User input
            input_event = input_stream.next().fuse() => {
                if let Some(Ok(event)) = input_event {
                    // Handle resize
                    if let crossterm::event::Event::Resize(w, h) = &event {
                        width = *w;
                        height = *h;
                        renderer.on_resize(width, height);
                        dirty = true;
                    }

                    let mut agent_restart: Option<String> = None;
                    let decoded = crate::interface::tui::input::decode::decode(event);

                    match decoded {
                        DecodedInput::Key(key) => {
                            let agent_state = AgentState {
                                is_running: app.transcript.is_running(),
                                has_draft: !app.composer.draft().is_empty(),
                                is_bang: app.composer.draft().starts_with('!'),
                                backtrack_primed: app.composer.backtrack_primed,
                            };

                            let resolved = resolve(&key, app.focus, &agent_state);

                            match resolved {
                                Action::Quit => {
                                    break;
                                }
                                Action::ClearComposer => {
                                    app.composer.clear();
                                }
                                Action::PrimeBacktrack => {
                                    app.composer.backtrack_primed = true;
                                }
                                Action::LoadLastUserMessage => {
                                    app.composer.backtrack_primed = false;
                                    if let Some(last_msg) = app.transcript.last_user_message() {
                                        app.composer.set_draft(last_msg);
                                    }
                                }
                                Action::QueueInput => {
                                    app.composer.handle_tab(app.transcript.is_running());
                                }
                                Action::TextAreaInput => {
                                    use crossterm::event::KeyEvent;
                                    let key_event = KeyEvent::new(key.code, key.modifiers);
                                    app.composer.textarea.input(key_event);

                                    let draft = app.composer.draft();
                                    app.composer.bang_shell = draft.starts_with('!');

                                    // Check for Enter submission
                                    if key.code == KeyCode::Enter
                                        && !app.transcript.is_running()
                                        && !draft.trim().is_empty()
                                    {
                                        let text = draft.trim().to_string();
                                        app.transcript
                                            .apply(crate::agent::r#loop::AgentEvent::MessageStart {
                                                role: "user".into(),
                                            });
                                        app.transcript
                                            .apply(crate::agent::r#loop::AgentEvent::TextDelta(
                                                text.clone(),
                                            ));
                                        app.transcript
                                            .apply(crate::agent::r#loop::AgentEvent::MessageEnd {
                                                role: "user".into(),
                                            });
                                        app.composer.clear();
                                        agent_restart = Some(text);
                                    }
                                }
                                Action::Submit(text) => {
                                    app.composer.clear();
                                    agent_restart = Some(text);
                                }
                                Action::CycleFocus => {
                                    app.focus = app.focus.cycle();
                                }
                                Action::ScrollUp(n) => {
                                    app.transcript.scroll_up(n);
                                }
                                Action::ScrollDown(n) => {
                                    app.transcript.scroll_down(n);
                                }
                                Action::None | Action::Dequeue | Action::InsertChar(_) => {}
                            }
                        }
                        DecodedInput::MousePress { .. } => {
                            app.focus = crate::interface::tui::state::FocusCtx::Transcript;
                        }
                        DecodedInput::MouseScroll { .. } => {}
                        DecodedInput::Resize { cols, rows } => {
                            width = cols;
                            height = rows;
                            renderer.on_resize(width, height);
                        }
                        _ => {}
                    }

                    if let Some(next) = agent_restart {
                        pending_prompt = Some(next);
                    }

                    dirty = true;
                }
            }

            // Branch 3: Frame timer (throttled redraw)
            _ = frame_interval.tick() => {
                if dirty {
                    let lines = crate::interface::tui::engine::compose_layout(
                        &app,
                        width,
                        height,
                        &model_name_owned,
                    );
                    let _ = renderer.render(&lines, width, height);
                    dirty = false;
                }
            }
        }
    }

    // ── Cleanup ─────────────────────────────────────────────────────
    restore_engine()
}

/// Setup for engine-based TUI: raw mode only (no alternate screen, no mouse capture).
fn setup_engine() -> io::Result<()> {
    enable_raw_mode()?;
    // Write initial clear + hide cursor
    let out = format!("{}{}", ansi::erase_screen(), ansi::cursor_goto(1, 1));
    io::stdout().write_all(out.as_bytes())?;
    io::stdout().flush()?;
    Ok(())
}

/// Restore terminal after engine TUI.
fn restore_engine() -> Result<(), String> {
    let out = format!("{}{}", ansi::show_cursor(), ansi::erase_screen());
    let _ = io::stdout().write_all(out.as_bytes());
    let _ = io::stdout().flush();
    disable_raw_mode().map_err(|e| format!("raw mode disable: {e}"))?;
    Ok(())
}

/// Get terminal size (columns, rows).
fn terminal_size() -> (u16, u16) {
    if let Ok((w, h)) = crossterm::terminal::size() {
        (w, h)
    } else {
        (80, 24)
    }
}
