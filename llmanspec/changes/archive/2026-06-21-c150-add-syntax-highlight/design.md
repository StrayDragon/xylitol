# c150-add-syntax-highlight: Design

## Architecture

```
src/infra/syntax/
├── mod.rs    # Public API: highlight(), supports_language()
├── theme.rs  # Syntax scope to ANSI formatter mapping
└── render.rs # Apply highlighting and render output
```

## Key Decisions

1. **Use `syntect` crate**: Rust-native syntax highlighting using Sublime Text .sublime-syntax files. Mature and well-maintained.
2. **Bundled themes**: Include dark and light theme mappings by default.
3. **Language auto-detection**: Use syntect's built-in detection or fall back to file extension.
4. **Output format**: Return `String` with ANSI escape codes for terminal rendering.
5. **Thread safety**: Syntect's `SyntaxSet` and `ThemeSet` are immutable after loading, safe for shared access.

## Feature Gate

- `infra-syntax` — optional feature
- Depends on `syntect` crate
