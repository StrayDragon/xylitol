# Bun demo external plugin

A minimal out-of-process plugin for xylitol. It speaks newline-delimited
JSON-RPC 2.0 over stdin/stdout and exposes two tools:

| tool | purpose |
|---|---|
| `echo` | returns the `text` argument |
| `add` | adds two integers |

Configure it in `config.yaml`:

```yaml
external_plugins:
  - name: "bun-demo"
    command: "bun"
    args: ["examples/plugins/bun-demo/plugin.ts"]
    trusted: true
    timeout_ms: 30000
```

The model sees the tools as `bun-demo__echo` and `bun-demo__add`.

## Protocol

Requests are one JSON object per line:

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocol_version":1,"plugin":"bun-demo"}}
```

The initialize result must declare `protocol_version: 1`, the `tools`
capability, and a tool list with `name`, `description`, and `input_schema`.

Calls use `tools/call`:

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"text":"hi"}}}
```

and return `{"content":"hi"}` in `result`.

## Run

```bash
bun examples/plugins/bun-demo/plugin.ts
```

The process stays alive until stdin closes. xylitol starts and stops it; do
not run it manually unless you are debugging the protocol.
