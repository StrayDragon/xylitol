# clap (quick reference)

`clap` is the standard Rust CLI argument parser (flags, options, subcommands, help text).

## Add dependency

```bash
cargo add clap --features derive
```

## Typical usage pattern

- Define a top-level `Cli` struct with `#[derive(Parser)]`
- Model major actions as `enum Commands` with `#[derive(Subcommand)]`
- Keep parsing separate from business logic (parse → dispatch)

## Bundled examples

This skill includes a small set of clap examples:

- `assets/examples/clap/`

## Upstream references

- docs.rs: https://docs.rs/clap/latest/clap/
- repo: https://github.com/clap-rs/clap
