# inquire (quick reference)

`inquire` provides interactive terminal prompts (text input, select, multiselect, confirm, password).

## Add dependency

```bash
cargo add inquire
```

## When to use

- You have a CLI tool that benefits from guided input (setup wizards, config generation).
- You want a nice prompt UX without building a full-screen TUI.

## Tips

- Always provide sensible defaults.
- Validate early and show clear validation messages.
- Offer a non-interactive mode (CLI args) for automation.

## Related references

- Key bindings: `references/INQUIRE_KEY_BINDINGS.md`
- docs.rs: https://docs.rs/inquire/latest/inquire/
- repo: https://github.com/mikaelmello/inquire
