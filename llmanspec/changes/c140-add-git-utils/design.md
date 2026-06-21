# c140-add-git-utils: Design

## Architecture

```
src/infra/git/
├── mod.rs    # Public API
├── repo.rs   # Git repository discovery (.git dir/file walk)
├── branch.rs # Current branch detection
└── url.rs    # Git URL parsing (SCP, HTTPS, SSH, git)
```

## Key Decisions

1. **Use `git2` crate** for branch detection and repo discovery (optional, `infra-git` feature).
2. **Manual .git walk** for path resolution without git2 (lighter alternative).
3. **Git URL parsing**: Pure Rust implementation using regex patterns for SCP-like (`git@host:path`), HTTPS, SSH, and git protocols.
4. **Branch watching**: Use `inotify` on Linux, `kqueue` on macOS, `ReadDirectoryChangesW` on Windows through the `notify` crate.

## Feature Gate

- `infra-git` — optional feature
- Depends on `git2` and `notify` crates
