# c125-add-clipboard: Design

## Architecture

```
src/infra/clipboard/
├── mod.rs       # Public API: copy_to_clipboard(), read_clipboard_image(), ClipboardImage
├── native.rs    # Platform-specific tool wrappers (pbcopy, clip, wl-copy, xclip, xsel)
├── osc52.rs     # OSC 52 terminal escape sequence support
└── image.rs     # Image clipboard reading (wl-paste, xclip, macOS, PowerShell)
```

## Platform Selection Strategy

1. **Native Rust crate** (`arboard`): Used when `infra-clipboard` feature is enabled and platform is macOS/Windows. Skipped on Linux due to Wayland/X11 ownership issues.
2. **Platform tools** (in order per platform):
   - macOS: `pbcopy`
   - Windows: `clip`
   - Linux/Wayland: `wl-copy`
   - Linux/X11: `xclip` → fallback `xsel`
   - Termux: `termux-clipboard-set`
3. **OSC 52 fallback**: Used for remote sessions (SSH) when native tools fail.

## Feature Flag

- `infra-clipboard` — optional feature, off by default
- Depends on `arboard` crate (macOS/Windows native clipboard)

## Key Decisions

1. **No native addon on Linux**: The `arboard` crate is X11-only and doesn't retain selection ownership properly on Wayland. Platform tools (wl-copy, xclip) are more reliable.
2. **OSC 52 with size limit**: 100KB base64 limit prevents terminal desynchronization on large payloads.
3. **Spawn + unref for wl-copy**: wl-copy hangs with synchronous exec, so we use async spawn with stdin pipe and unref.
