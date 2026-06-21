# c135-add-shell-process-mgmt: Design

## Architecture

```
src/infra/process/
├── mod.rs    # Public API
├── shell.rs  # bash discovery, shell config, shell env
├── group.rs  # Process tree kill (cross-platform)
└── child.rs  # wait_for_child with pipe drain protection
```

## Key Decisions

1. **No feature gate**: Built-in module since it integrates with existing bash_executor.
2. **Windows Git Bash detection**: Search ProgramFiles/ProgramFiles(x86) for Git/bin/bash.exe.
3. **Unix bash discovery**: Try /bin/bash first, then `which bash`, then fallback to sh.
4. **Process tree kill**: On Unix use negative PID (process group); on Windows use `taskkill /F /T`.
5. **wait_for_child**: After process exit, re-arm a grace timer on each data chunk from stdout/stderr. Release after 100ms of inactivity or when both pipes end.
