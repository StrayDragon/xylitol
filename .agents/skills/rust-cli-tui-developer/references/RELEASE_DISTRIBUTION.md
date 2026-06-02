# Rust CLI/TUI Release & Distribution

## Release build profile (Cargo.toml)

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

## Build commands

```bash
cargo build --release
```

Cross compilation requires target toolchains:

```bash
cargo build --target x86_64-unknown-linux-musl --release
cargo build --target aarch64-apple-darwin --release
```

## GitHub Actions (example)

```yaml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build binary
        run: cargo build --release --target x86_64-unknown-linux-musl
```
