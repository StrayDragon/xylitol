# Deferred / future — c445

## Deferred Items

- Full overlay focus-restore state machine (pi eligible / blocked / resume)
- `InputListener` pipe if product slash/debug keys need pre-focus intercept (revisit when wiring `src/app/tui`)
- Alternate-screen mode (explicitly out of product UX; scrollback model stays)

## Branch Options

- Feature-gate `terminal_image` encode behind `image` feature vs delete outright — prefer delete if no in-tree consumer after trim

## Triggers to Reopen

- Product TUI needs nested modal focus restore across multiple overlays
- Product needs Kitty/iTerm image paste or screenshot inline display
