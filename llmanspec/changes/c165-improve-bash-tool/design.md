# c165-improve-bash-tool: Design

## Changes

### 1. `BashOperations` trait in `src/agent/tools/bash.rs`
```rust
#[async_trait]
pub trait BashOperations: Send + Sync {
    async fn execute(
        &self,
        command: &str,
        cwd: &Path,
        on_chunk: OnChunkCallback<'_>,
        abort: Arc<AtomicBool>,
    ) -> Result<BashResult>;
}
```

### 2. Spawn hooks in `src/agent/bash_executor.rs`
```rust
pub struct BashHooks {
    pub pre_spawn: Option<Box<dyn Fn(&str) + Send + Sync>>,
    pub post_spawn: Option<Box<dyn Fn(&BashResult) + Send + Sync>>,
}
```

### 3. Graduated timeout
- Initial timeout fires → send SIGTERM
- 5-second grace period → send SIGKILL
- Uses c135's `kill_process_tree()` for Unix process groups

### 4. Environment injection
- Auto-add agent bin directory to PATH in shell env
- Uses c135's `build_shell_env()`

## Dependencies

- Depends on c135 (shell process management)
