# Xylitol Architecture

> Single Source of Truth · Last updated: 2026-06-12
> 75 source files, ~13,000 LoC, 322 tests (245 lib + 77 BDD)

## 1. Project Identity

**Xylitol = Minimalist Agent Runtime** — a lean, single-shot agent execution engine:
ReAct loop + 7 built-in tools + CLI. It is NOT a platform, NOT an orchestration
layer, NOT a multi-user service.

### Explicitly Out of Scope (permanent)

| Category | Reason |
|----------|--------|
| PackageManager detection (npm/pnpm/yarn/bun) | User manages dependencies |
| OAuth / auth-storage / token flows | User configures API keys manually |
| Extensions SDK / plugin system | Out of scope |
| Additional providers (Gemini, Ollama, etc.) | OpenAI-like + Anthropic-like only |
| Multi-modal inputs (images, documents) | Not planned |
| TUI / GUI / Web interface | CLI single-shot only |

### Provider Policy

Two provider interfaces are supported:
- **OpenAI-compatible API** — any endpoint speaking OpenAI chat completions
- **Anthropic-compatible API** — any endpoint speaking Anthropic messages

No built-in model lists, no auth flows, no provider auto-discovery.
Users provide their own API keys and endpoints.

---

## 2. Three-Layer Architecture

```
src/
  interface/     ← CLI entry, print output, diff review
  agent/         ← Core: ReAct loop, session, tools, providers
  infra/         ← Support: config, hooks, session persistence, skills
```

### Layer Dependency Graph

```
interface/
  ├── cli         → agent(loop, session, model, registry, tools), infra(session)
  ├── print       → agent(loop)
  └── diff_review → infra(config)

agent/
  ├── loop        → agent(session, event, trust, provider), infra(session)
  ├── session     → infra(session, compaction)
  ├── model       → agent(provider)
  ├── registry    → agent(model, session)
  ├── resolver    → agent(registry, model)
  ├── tools/*     → agent(traits, error, truncate, mutation)
  └── provider/*  → agent(traits, types, error)

infra/
  ├── config      → agent(model, profile)
  ├── hooks       → standalone
  ├── session     → standalone (JSONL storage, compaction)
  ├── skills      → standalone (MCP)
  └── resource    → standalone (AGENTS.md walk-up)
```

**Rule**: dependencies flow downward. `agent/` never imports from `interface/`.
`infra/` never imports from `agent/` or `interface/`.

---

## 3. Visibility Policy

- **`pub`**: only true API contracts consumable by external crates or BDD tests
  (traits `XyModel`/`XyTool`, config types, AgentSession core methods)
- **`pub(crate)`**: everything else — internal implementation details
- **`::` nesting**: module trees are `pub(crate) mod` unless direct consumers exist

Current state: **185 `pub` : 134 `pub(crate)`** (down from 335:1)

---

## 4. Agent Loop — ReAct Execution

### Core Flow

```
CLI → AgentLoop::run(prompt, session_id)
       ├── [Turn N] Build messages (system prompt + history)
       ├── [Turn N] Call LLM provider (streaming with CancellationToken)
       ├── [Turn N] Parse response → TextDelta / ToolCall
       ├── [Turn N] Execute tool (if tool call)
       ├── [Turn N] Check compaction (token budget > threshold)
       ├── [Turn N] Apply trust/security checks
       └── [Turn N+1] Repeat until AgentEnd or max iterations
```

### Event Stream

`AgentLoop::run()` returns `impl Stream<Item = AgentEvent>` with these variants:

| Event | Description |
|-------|-------------|
| `TurnStart` | New turn begins |
| `MessageStart` / `MessageUpdate` / `MessageEnd` | Streaming LLM response |
| `TextDelta` / `ThinkingDelta` | Content chunks |
| `ToolExecutionStart` / `ToolExecutionEnd` | Tool lifecycle |
| `CompactionStart` | Context compaction trigger |
| `ModelSelect` | Model change notification |
| `Error` | Error with message |
| `AgentEnd` | Execution complete |

### Retry

On transient failures (rate limits, timeouts), the loop retries with
`RetryState` tracking attempt count. Retry strategy: linear backoff,
max 3 attempts by default.

---

## 5. Seven Built-in Tools

All tools implement `XyTool` trait (async, JSON args, string result).

| Tool | File | Description |
|------|------|-------------|
| `read` | `tools/read.rs` | Read file with offset/limit, truncation |
| `write` | `tools/write.rs` | Write file, `FileMutationQueue` for serialization |
| `edit` | `tools/edit.rs` | Precise text replacement, `FileMutationQueue` |
| `bash` | `tools/bash.rs` | Shell command execution with timeout |
| `grep` | `tools/grep.rs` | Regex search with glob pattern, line truncation |
| `find` | `tools/find.rs` | File search with glob pattern, truncation |
| `ls` | `tools/ls.rs` | Directory listing with metadata |

### ToolRegistry

Manages tool discovery. `ToolRegistry::builtins()` returns all 7 tools.
`ToolRegistry::filtered()` returns a subset by name.

### Safety

- **`FileMutationQueue`**: serializes write/edit operations per file path
- **`path_utils`**: resolves relative paths to CWD, validates paths
- **`OutputAccumulator`**: streaming buffer with temp file spillover for large tool outputs

---

## 6. Provider System

### Model Resolution Chain

```
User prompt "use gpt-4o"
  → AgentSession::select_model("gpt-4o")
    → ModelRegistry::find("gpt-4o")
      → ModelMeta (config with kind, api_key, model, base_url)
        → ModelConfig::into_provider()
          → OpenAIProvider / AnthropicProvider
            → XyModel trait (generate_stream)
```

### Provider Implementations

| Provider | API | Streaming | Requirements |
|----------|-----|-----------|--------------|
| `OpenAIProvider` | Chat Completions (SSE) | Native SSE | `OPENAI_API_KEY` env var |
| `AnthropicProvider` | Messages API (SSE) | eventsource-stream | `ANTHROPIC_API_KEY` env var |
| `FakeProvider` | No API call | Scenario-based | `dev-fake-provider` feature |
| `MockXyModel` | Returns fixed text | No streaming | `#[cfg(test)]` only |

### Cancellation

All provider streams accept `CancellationToken`. The loop uses `tokio::select!`
to cancel LLM calls on abort or timeout.

---

## 7. Session System

### Persistence

Sessions are stored as **JSONL files** in `~/.local/share/xylitol/sessions/`.
Each entry is a `SessionEntry` variant:

| Variant | Description |
|---------|-------------|
| `MessageEntry` | LLM message (user/assistant/system) |
| `CompactionEntry` | Context compaction checkpoint |
| `BranchSummaryEntry` | Branch summary for forked sessions |
| `ModelChangeEntry` | Model switch event |
| `ThinkingLevelChangeEntry` | Thinking level change |
| `CustomEntry` / `CustomMessageEntry` | User-defined metadata |

### SessionManager

- `snapshot()` — persist current session state
- `restore()` — restore session by ID
- `list()` — enumerate all sessions
- `spawn()` — fork a session (subset of history)
- `prune()` — garbage collect old sessions
- `diff()` — compute diff between sessions
- `merge()` — merge two sessions
- `compact()` — compact session with LLM summarization

### Compaction

When token budget exceeds threshold (default 80%), triggers context compaction:
1. Find cut point in conversation history
2. Generate LLM summary of prefix
3. Replace prefix with `CompactionEntry`
4. Append file operations XML to summary

---

## 8. Hook System

### Architecture

```
Agent Action → HookDispatcher → [global hooks] → [project hooks] → [user hooks]
                                  ↓
                          DispatchResult { action, output }
```

### Hook Phases

| Phase | Timing | Effect |
|-------|--------|--------|
| `Pre` | Before action | Can block or modify input |
| `Post` | After action | Can modify output |
| `ReviewStart` | Before diff review | Receives diffs |
| `ReviewEnd` | After diff review | Receives verdict |

### Hook Types

- **Shell scripts**: `run_hook_script()` executes external scripts
- **Inline hooks**: defined in `~/.config/xylitol/hooks.yaml` or `<project>/.xylitol/hooks.yaml`

---

## 9. Trust & Security

### TrustManager (`agent/trust.rs`, 529L)

Manages project-level trust decisions with persistent storage.

- `TrustStore` — persistent key-value store (`~/.local/share/xylitol/trust.json`)
- `TrustOption` — discovered trust input (AGENTS.md, .xylitol config dir)
- Path resolution: walks up directory tree for trust indicators

### ProjectTrust (`agent/project_trust.rs`, 394L)

Resolves whether a project directory has trust inputs:
- `ResolveTrustOptions` — configuration for trust resolution
- `TrustResolution` — resolution result (trusted/not trusted + reason)
- `format_trust_prompt()` — generates user trust prompt

### Security Config

- **Bash timeout**: configurable per-execution timeout (default 120s)
- **Filesystem**: path validation (no escapes outside project root)
- **Network**: rate limiting placeholder
- **Resource limits**: memory/CPU/disk limits in config

---

## 10. Config System

### Loading Order (lowest → highest priority)

1. Global base: `~/.config/xylitol/config.yaml`
2. Global local: `~/.config/xylitol/config.local.yaml`
3. Project base: `<project>/.xylitol/config.yaml`
4. Project local: `<project>/.xylitol/config.local.yaml`
5. CLI override: `--config <path>`

### AppConfig Structure

```yaml
model:
  default_model: gpt-4o
  models:
    gpt-4o:
      provider: openai
      model: gpt-4o
      base_url: ~
    claude:
      provider: anthropic
      model: claude-sonnet-4-20250514

agents:
  default_profile: default
  profiles:
    default:
      model: gpt-4o
      system_prompt: ~
      max_iterations: 50

execution:
  model: ~
  system_prompt: ~
  max_retries: 3

hooks:
  global: []
  project: []
  user: []

security:
  enabled: false
  bash:
    timeout_secs: 120

repeat_detection:
  enabled: false
  min_n: 2
  recovery:
    strategy: sequential

review:           # feature: ui-review
  backend: cli
  mode: on-step

tools: {}
```

### API Key Resolution

```
ModelConfig::resolve_model(model_id)
  ├── Lookup in config.model.models
  ├── Determine ModelKind (OpenAi / Anthropic)
  ├── Read env var: OPENAI_API_KEY / ANTHROPIC_API_KEY
  └── Return ModelConfig { kind, api_key, model, base_url }
```

### Secrets

Supports `secret.env` files (dotenv format) in config directories for
sensitive values not in YAML.

---

## 11. Diff Review System

### Architecture

```
ReviewEngine
  ├── from_config(ReviewConfig)
  ├── create_session(files) → ReviewSession
  └── run_review(session) → ReviewVerdict { AcceptAll | RejectWithComments }
```

### CLI Backend (`ui-review` feature, ratatui)

Full-screen terminal interface with:
- Unified diff rendering (color-coded additions/deletions)
- Line-level commenting with severity (info/warning/error/critical)
- Keyboard navigation (vim-style: j/k/g/G/n/N)
- Accept all / reject with comments
- Help overlay (`?` key)

### Diff Generation

Uses `similar` crate for line-level diffs, converted to structured
`DiffHunk` → `DiffLine` types. Hunk-aware scrolling and comment
attachment to specific file:line ranges.

---

## 12. Feature Flags

| Feature | Layer | Default | Description |
|---------|-------|---------|-------------|
| `infra-skills` | infra | ✅ | MCP skill integration |
| `infra-session` | infra | ✅ | Session persistence (JSONL, tree, fork) |
| `ui-review` | interface | ✅ | Diff review rendering (ratatui) |
| `agent-planning` | agent | — | Planning orchestration |
| `agent-model-lock` | agent | — | Model locking |
| `infra-lsp` | infra | — | LSP integration (lspz) |
| `infra-dap` | infra | — | Debug adapter protocol |
| `infra-acp` | infra | — | Agent Client Protocol |
| `infra-sandbox` | infra | — | Sandbox execution |
| `infra-rtk` | infra | — | Real-time kernel |
| `dev-fake-provider` | dev | — | Fake provider for testing |
| `dev-e2e` | dev | — | End-to-end tests |

---

## 13. Dependency Map

### Direct Dependencies (31 crates)

| Crate | Usage |
|-------|-------|
| `tokio` | Async runtime (rt-multi-thread, fs, process, sync, time) |
| `serde` / `serde_json` | Serialization |
| `yaml_serde` | YAML config parsing |
| `clap` | CLI argument parsing |
| `reqwest` | HTTP client (stream + json) |
| `async-openai` | OpenAI SDK |
| `eventsource-stream` | SSE parsing for Anthropic |
| `async-stream` | Stream generators |
| `async-trait` | Async trait support |
| `thiserror` / `anyhow` | Error handling |
| `tracing` | Structured logging |
| `ratatui` / `crossterm` | TUI rendering (optional) |
| `similar` | Diff generation |
| `minijinja` | Template rendering |
| `uuid` | Session IDs |
| `chrono` | Timestamps |
| `sha2` | Content hashing |
| `zstd` | Compression (session storage) |
| `hex` | Hash encoding |
| `futures` | Stream combinators |
| `tokio-util` | CancellationToken |
| `schemars` / `jsonschema` | Config schema validation |
| `dirs` | XDG path resolution |
| `regex` | Grep pattern matching |
| `glob` | File pattern matching |
| `lspz` | LSP integration (optional) |
| `rmcp` | MCP client (optional) |
| `agent-client-protocol` | ACP (optional) |

### Removed Dependencies (audit phase)

`dotenvy`, `rmp-serde`, `shlex`, `vt100`, `serde_yaml` (replaced by `yaml_serde`)

---

## 14. Unsafe Code Policy

**Zero `unsafe` in production code.** All 13 `unsafe` invocations are confined
to test code in `infra/config/`, mutating `std::env::set_var` / `remove_var`.

Safety is ensured by:
1. `ENV_LOCK` — global `Mutex` serializing all env-modifying tests
2. `EnvGuard` — records original values on construction, restores on drop
3. Every call site annotated with `// SAFETY:` comment

---

## 15. Test Strategy

### Test Tiers

| Tier | Count | Runner | Description |
|------|-------|--------|-------------|
| Lib unit tests | 245 | `cargo test --lib` | Per-module unit tests |
| BDD scenarios | 77 | `cargo test --test bdd -- --test-threads=1` | rstest-bdd feature tests |
| Doc tests | ~0 | `cargo test --doc` | `///` examples |

### BDD Framework

Uses `rstest-bdd` with `.feature` files in `tests/features/`. Step definitions
in `tests/bdd.rs`. Scenarios cover all 7 built-in tools + agent loop + hooks +
session persistence.

### Key Commands

```bash
just test   # cargo test (or cargo nextest)
just qa     # fmt-check + lint + test + doc-check + prek
just lint   # cargo clippy
just fmt    # cargo fmt
```

---

## 16. Module Inventory

| Layer | Module | Lines | Description |
|-------|--------|-------|-------------|
| agent | `loop.rs` | 470 | ReAct loop + AgentEvent stream |
| agent | `session.rs` | 700+ | AgentSession lifecycle |
| agent | `trust.rs` | 529 | TrustStore + trust file management |
| agent | `project_trust.rs` | 394 | Project trust resolution |
| agent | `registry.rs` | 290 | ModelRegistry + ProviderConfig |
| agent | `resolver.rs` | 310 | Model resolution (exact/fuzzy/fallback) |
| agent | `tools/mod.rs` | 140 | ToolRegistry |
| agent | `tools/edit.rs` | 390 | Edit tool |
| agent | `tools/grep.rs` | 270 | Grep tool |
| agent | `tools/bash.rs` | 180 | Bash tool |
| agent | `tools/read.rs` | 160 | Read tool |
| agent | `tools/write.rs` | 170 | Write tool |
| agent | `tools/find.rs` | 200 | Find tool |
| agent | `tools/ls.rs` | 130 | Ls tool |
| agent | `tools/truncate.rs` | 310 | Output truncation |
| agent | `tools/accumulator.rs` | 320 | OutputAccumulator |
| agent | `provider/openai.rs` | 200 | OpenAI provider |
| agent | `provider/anthropic.rs` | 220 | Anthropic provider |
| infra | `config/types.rs` | 890 | AppConfig + all config types |
| infra | `config/loader.rs` | 360 | Multi-layer config loader |
| infra | `hooks/dispatcher.rs` | 180 | Hook dispatcher |
| infra | `session/compaction.rs` | 1230 | LLM compaction + summarization |
| infra | `session/manager.rs` | 790 | SessionManager (CRUD, fork, merge) |
| infra | `session/types.rs` | 200 | SessionEntry + header types |
| infra | `session/storage.rs` | 140 | JSONL encode/decode with zstd |
| interface | `cli/mod.rs` | 110 | CLI argument parsing |
| interface | `print.rs` | 55 | Print mode output |
| interface | `diff_review/cli.rs` | 840 | ratatui diff review |
| interface | `diff_review/mod.rs` | 440 | ReviewEngine + diff generation |
| interface | `diff_review/types.rs` | 150 | Review data structures |

---

## 17. Coding Conventions

- **Edition**: Rust 2024, stable toolchain
- **Errors**: `thiserror` for library error enums, `anyhow` at boundaries
- **Logging**: `tracing` (structured), not `log`
- **Async**: `tokio` multi-threaded runtime
- **Formatting**: 4 spaces, max width 100 (`rustfmt.toml`)
- **Lints**: clippy with `-D warnings` in CI
- **Commits**: Conventional Commits (`type(scope): description`)
