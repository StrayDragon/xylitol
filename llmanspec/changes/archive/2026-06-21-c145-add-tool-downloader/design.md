# c145-add-tool-downloader: Design

## Architecture

```
src/infra/tools/
├── mod.rs       # Public API: ensure_tool(), get_tool_path()
├── downloader.rs # HTTP download, GitHub API version query
├── extract.rs   # tar.gz and zip archive extraction
└── platform.rs  # Platform detection, asset name mapping
```

## Key Decisions

1. **GitHub API for version**: Fetch latest release from `api.github.com/repos/{owner}/{repo}/releases/latest`.
2. **Platform mapping**: Map Rust `std::env::consts::ARCH` and `std::env::consts::OS` to GitHub asset names.
3. **Extraction**: Use `flate2` + `tar` for tar.gz; use `zip` crate for zip.
4. **Binary search**: After extraction, recursively search for the binary in extracted files (handles versioned subdirectories).
5. **Offline mode**: Skip download when `PI_OFFLINE=1` env var is set.
6. **Tools supported**: `fd` (sharkdp/fd) and `rg` (BurntSushi/ripgrep).

## Feature Gate

- `infra-tools` — optional feature
