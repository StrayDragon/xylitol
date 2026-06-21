# c155-add-platform-utils: Design

## Architecture

```
src/
├── infra/
│   ├── update/mod.rs       # Version check: HTTP GET to API, semver comparison
│   ├── changelog/mod.rs    # CHANGELOG.md parsing: ## header scanning
│   ├── fs-watch/mod.rs     # File watcher: notify crate wrapper
│   └── browser/mod.rs      # Open URL: opener crate (macOS open, xdg-open, Windows start)
├── infra/frontmatter/       # Enhance existing: serde_yaml extraction from --- blocks
```

## Key Decisions

1. **Version check**: `check_for_new_version()` queries a configurable API endpoint. Returns `Option<VersionInfo>` with version and optional note.
2. **Changelog parsing**: Simple lexer scanning for `## [x.y.z]` headers. Returns `Vec<ChangelogEntry>`.
3. **File watching**: Use `notify` crate with debounced events and retry on error.
4. **Browser open**: Use `open` crate or shell commands (`open` on macOS, `xdg-open` on Linux, `start` on Windows).
5. **Frontmatter enhancement**: Extract existing logic from skills loader into reusable `parse_frontmatter()`.

## Feature Gate

- `infra-platform` — optional feature for update, changelog, fs-watch, browser
- Frontmatter enhancement: built-in (already partially in skills loader)
