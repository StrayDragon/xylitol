# xylitol

An LLM-driven local development agent for Rust projects. Provides a CLI-based
agent with tool-calling capabilities (bash, file I/O, grep, find)
governed by a configurable security policy.

## Quick Start

```bash
# Clone and build
git clone https://github.com/straydragon/xylitol
cd xylitol
cargo build --release

# Run with default features
# Requires a prompt: xylitol "your prompt"
cargo run -- --model <alias> -- "your prompt"
```

## Configuration

Place a `xylitol.yaml` (or `.xylitol.yaml`) in your project root or
`$XDG_CONFIG_HOME/xylitol/`. See [`configs/example.yaml`](configs/example.yaml)
for all available options.

Key sections:

| Section     | Purpose                              |
|-------------|--------------------------------------|
| `model`     | Default model & provider aliases     |
| `execution` | Max steps, context window limits     |
| `security`  | Bash/filesystem/network policies     |
| `tools`     | Allowlist, blocklist, limits         |
| `hooks`     | Pre/post tool-call hook scripts      |

## Feature Flags

| Feature           | Default | Description                         |
|-------------------|---------|-------------------------------------|
| `agent-planning`  | yes     | Multi-step planning orchestrator    |
| `agent-model-lock`| yes     | Model lock file support             |
| `infra-lsp`       | yes     | Language Server Protocol pool       |
| `infra-dap`       | yes     | Debug Adapter Protocol layer        |
| `infra-acp`       | yes     | Agent Client Protocol support       |
| `infra-skills`    | yes     | Skill/MCP server integration        |
| `infra-session`   | yes     | Session persistence & compaction    |
| `infra-sandbox`   | yes     | Sandboxed execution environment     |
| `infra-rtk`       | yes     | Runtime toolkit utilities           |

| `ui-review`       | yes     | Diff review UI mode                 |

Build with minimal features:

```bash
cargo build --no-default-features
```

## Development

```bash
just setup   # Install git hooks
just fmt     # Format code
just lint    # Run clippy
just test    # Run tests
just qa      # Full quality check
```

## License

MIT
