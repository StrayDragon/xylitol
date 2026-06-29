# c290-restructure-app-layer — Design

## 0. Problem statement

After c285, the source tree has a clean `domain/` / `runtime_protocol/` / `agent/` / `infra/` split, but the client-facing surface is still organized around an old concept:

- `interactive/` hosts the CLI composition root, RPC transport, print mode, and TUI code.
- `server/` is a second, top-level composition root that is logically an "app mode" rather than a core runtime.
- `protocol.rs` is a single file that will need to grow REST/WS/envelope submodules.
- CLI, RPC, and Server each duplicate the same `Agent::with_ports(...)` wiring.

The goal is to reorganize the surface layer into a single `app/` directory that contains all entry points (CLI, RPC, print, server, TUI, GUI), split `protocol.rs` into a `protocol/` directory, and centralize Agent construction in `app::composition`.

The crate remains single-package; we use Cargo features to avoid compiling heavy app-mode dependencies when they are not needed.

## 1. New module map

```text
src/
├── lib.rs
├── main.rs
├── tests.rs                 # arch_guard + unit-test wiring
│
├── domain/                  # pure vocabulary + errors
├── runtime_protocol/        # agent↔infra boundary traits
├── protocol/                # client↔core wire vocabulary
│   ├── mod.rs
│   ├── command.rs
│   ├── event.rs
│   └── transport.rs
├── agent/                   # orchestration: ReAct loop, session, registries
│   ├── mod.rs
│   ├── facade.rs
│   ├── runtime/
│   ├── session/
│   ├── tools/
│   ├── model/
│   ├── prompt/
│   └── compaction/
│
├── infra/                   # runtime implementations (internal org unchanged)
│
└── app/                     # all user-facing entry points
    ├── mod.rs
    ├── driver.rs            # Driver trait + InProcessDriver
    ├── composition.rs       # shared Agent construction
    ├── print.rs             # print mode
    ├── rpc.rs               # stdio RPC transport
    ├── cli/                 # CLI composition root
    │   ├── mod.rs
    │   ├── args.rs
    │   ├── provider_guidance.rs
    │   └── run.rs
    ├── server/              # always-on server app
    │   ├── mod.rs
    │   ├── runtime.rs
    │   ├── rest.rs
    │   ├── ws.rs
    │   └── lock.rs
    ├── tui/                 # terminal UI (feature-gated)
    │   └── diff_review/
    └── gui.rs               # future GUI placeholder (feature-gated)
```

## 2. Dependency direction

```text
app → agent → runtime_protocol → domain
  ↓     ↑
  └──── infra ───────────────────┘
protocol ───────────────────────→ domain
```

Hard invariants (same as HC-1/HC-2):

- `domain` has zero crate-internal dependencies.
- `runtime_protocol` depends only on `domain`.
- `agent` depends on `domain` + `runtime_protocol`; it never imports concrete `infra` types.
- `infra` depends on `domain` + `runtime_protocol`; it never imports `agent`.
- `app` is the only layer that may import both `agent` and `infra` together, and only inside composition-root modules.
- `protocol` depends only on `domain` (and serde).

## 3. Feature matrix

| Feature | Enables | Extra deps | Default |
|---|---|---|---|
| `cli` | `app::cli` | clap | yes |
| `rpc` | `app::rpc` | — | yes (bundled with `cli`) |
| `server` | `app::server` | axum, tokio-tungstenite, tower-http | no |
| `tui` | `app::tui` | ratatui, crossterm | no |
| `gui` | `app::gui` | tauri (future) | no |

`app/mod.rs` gates each submodule with `#[cfg(feature = "...")]`.

## 4. `app::composition` design

Responsibility: construct a fully-wired `Agent` from a small options struct.

```rust
pub struct BuildAgentOptions {
    pub cwd: String,
    pub config_path: Option<PathBuf>,
    pub system_prompt: Option<String>,
    pub context_files: Vec<(String, String)>,
    pub append_system_prompt: Vec<String>,
    pub max_iterations: u32,
    pub compaction_threshold: f64,
    pub compaction_settings: Option<CompactionSettings>,
    pub session_id: Option<String>,
}

pub fn build_agent(options: BuildAgentOptions) -> Result<Agent, String> {
    let secret_resolver = Arc::new(InfraSecretResolver::new());
    let mut model_registry = ModelRegistry::new(secret_resolver);
    // ... load config / env models ...

    let tool_registry = ToolRegistry::from_tools(infra::tools::default_tools());
    let store: Arc<dyn SessionStore> = Arc::new(SessionManager::...);
    let sink: Arc<dyn EventSink> = Arc::new(EventBus::new());
    let bash_executor: Arc<dyn BashExecutor> = Arc::new(InfraBashExecutor::new());
    let export_io: Arc<dyn ExportIo> = Arc::new(StdExportIo::new());

    let mut agent = Agent::with_ports(
        model_registry,
        tool_registry,
        store,
        sink,
        options.system_prompt,
        options.context_files,
        options.append_system_prompt,
        options.max_iterations,
        options.compaction_threshold,
        options.cwd,
        options.compaction_settings,
        Arc::new(infra::provider::factory::build_provider),
        infra::sandbox::noop_engine(),
        bash_executor,
        export_io,
    );

    if let Some(id) = options.session_id {
        agent.session_mut().set_session(id);
    }

    Ok(agent)
}
```

CLI, RPC, Server, and TUI call `build_agent(...)` with their own `BuildAgentOptions`. Special cases (e.g., server wants a custom `EventSink`) are handled by wrapping the returned `Agent` or by adding optional fields to `BuildAgentOptions`.

## 5. `protocol/` split

- `protocol/mod.rs` re-exports `Command`, `Event`, and any transport helpers.
- `protocol/command.rs` owns `Command` and `impl Command`.
- `protocol/event.rs` owns `Event`.
- `protocol/transport.rs` is the future home of `Envelope`, `Frame`, `Seq`, and shared serde helpers used by RPC/WS/REST.

No public API break: callers still use `crate::protocol::Command` / `crate::protocol::Event`.

## 6. Migration mechanics

### 6.1 `interactive/` → `app/`

- `git mv src/interactive src/app`
- Update `src/lib.rs` module declarations.
- Global replace `crate::interactive::` → `crate::app::`.
- Move `src/app/diff_review/` → `src/app/tui/diff_review/`.

### 6.2 `server/` → `app/server/`

- `git mv src/server src/app/server`
- Global replace `crate::server::` → `crate::app::server::`.
- Update `src/lib.rs`: remove `pub mod server;`.

### 6.3 `protocol.rs` → `protocol/`

- Create `src/protocol/`.
- Split `Command` into `protocol/command.rs`, `Event` into `protocol/event.rs`.
- Add `protocol/transport.rs` placeholder.
- `src/protocol/mod.rs` re-exports everything.
- Delete `src/protocol.rs`.

### 6.4 Create `app::composition`

- Extract common wiring from `app::cli::run`, `app::rpc::run`, and `server::runtime::start`.
- Define `BuildAgentOptions` and `build_agent`.
- Replace duplicated wiring in all three composition roots with calls to `build_agent`.

### 6.5 Feature gates

- Update `Cargo.toml` `[features]` block.
- Wrap `app::server`, `app::tui`, `app::gui` with `#[cfg(feature = "...")]`.
- Rename `ui-review` → `tui` everywhere.
- Make `main.rs` default to `app::cli::run()` when `cli` feature is enabled; provide stubs or compile errors for missing default features.

### 6.6 arch_guard update

- Scan path changes:
  - `src/interactive/` → `src/app/`
  - `src/server/` → `src/app/server/`
- Exempt composition roots from the "no agent/infra import" rule:
  - `app/cli/`
  - `app/server/`
  - `app/rpc.rs`
  - `app/composition.rs`
  - `app/print.rs` (read-only stream matching)
- `app/driver.rs` must remain free of `crate::infra` imports.
- The existing `agent → infra` and `infra → agent` guards stay unchanged.

## 7. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Large import-path blast radius | High churn | `cargo build` after each mechanical move; grep for `crate::interactive::` and `crate::server::` before final validation |
| Feature gate breaks default build | `cargo run -- --help` fails | Add `cli` to default features; run `cargo check` with and without `--no-default-features` |
| `app::composition` becomes a god file | Hard to coordinate across modes | Keep it as pure wiring; use `BuildAgentOptions` struct; add `composition/` submodules only if it grows beyond ~400 lines |
| `arch_guard` exempt list drifts | New app modules bypass rules | Document exemptions in the guard and review after each new app mode |
| TUI feature rename breaks existing scripts | Users with `--features ui-review` fail | Update `justfile`, README, and CI; no backward shim per project rule |

## 8. Validation strategy

1. `cargo fmt --check`
2. `cargo clippy --lib --features cli`
3. `cargo clippy --lib --features server`
4. `cargo clippy --lib --features tui`
5. `cargo test --lib`
6. `cargo test --test bdd -- --test-threads=1`
7. `cargo test --lib arch_guard`
8. `cargo doc --no-deps --all-features`
9. `llman sdd validate c290-restructure-app-layer --strict --no-interactive`
10. `llman sdd validate --all`

All checks must be green before archiving.
