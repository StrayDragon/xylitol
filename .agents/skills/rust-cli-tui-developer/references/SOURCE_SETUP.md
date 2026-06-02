# Bundled Examples & Upstream Sources

This skill does **not** include full upstream git submodules. It bundles a small set of examples and references to keep context lightweight.

## What’s included locally

- Clap examples (Rust): `assets/examples/clap/`
- Ratatui UI recordings/assets: `assets/examples/ratatui/` (VHS tapes, README)
- Curated notes: `references/` (architecture, key bindings, readme snapshots)

Quick verification:

```bash
ls assets/examples/clap/
ls assets/examples/ratatui/
```

## If you need full upstream sources

Clone the upstream repos from `metadata.*_repo` (see `rust-cli-tui-developer/SKILL.md`) and use them as the authoritative reference for APIs and the latest examples.
