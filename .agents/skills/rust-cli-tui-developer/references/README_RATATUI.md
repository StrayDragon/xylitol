# ratatui (quick reference)

`ratatui` is a Rust framework for full-screen terminal user interfaces (layout, widgets, styling) typically paired with `crossterm` for event handling.

## Add dependencies

```bash
cargo add ratatui crossterm
```

## When to use

- You need rich layouts, tables/lists, live updates, or keyboard-driven navigation.
- A simple prompt-based flow (`inquire`) is not enough.

## Architecture reminders

- Separate **app state** from **rendering**.
- Use an event loop (read events → update state → draw).
- Handle resize and ensure terminal restore on exit/panic.

## Bundled materials

- Ratatui UI recordings/assets: `assets/examples/ratatui/`
- Architecture notes: `references/RATATUI_ARCHITECTURE.md`

## Upstream references

- docs.rs: https://docs.rs/ratatui/latest/ratatui/
- repo: https://github.com/ratatui/ratatui
