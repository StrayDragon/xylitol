# Tasks

- [x] Update delta spec (`tui-interface`) for: initial paint + full-frame render + clean exit
- [x] Ensure initial draw before event loop
- [x] Fix render strategy to always render full frame on `Terminal::draw`
- [x] Fix Ctrl+D exit by avoiding long-lived `spawn_blocking` input loop
- [x] Run `llman sdd validate c105-fix-tui-initial-render --strict --no-interactive`
- [x] Run `just qa`
- [ ] Archive: `llman sdd archive run c105-fix-tui-initial-render`
