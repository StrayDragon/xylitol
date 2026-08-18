# Design: 会话域拆分与 flatten 观测

## 1. Session vs Store

```text
XyError::Session(XySessionError)
                 ├── Store(XyStoreError)     // JSONL / port
                 ├── NoActiveSession         // runtime bind
                 ├── Busy { message }        // mutation while worker
                 └── EntryNotFound { id }    // in-memory tree travel

XySessionStore::* -> Result<_, XyStoreError>   // 不变
```

`XySessionError::kind()`：Store 臂委托内层（`NotFound`/`Io`/…）；控制面臂用自己的 variant 名。

Driver flatten（产品 `kind` 不变）：

| 来源 | Driver `kind()` | `detail_kind` / `source.kind` |
|---|---|---|
| Store NotFound / Session NoActive / tree EntryNotFound | NotFound | Session |
| Store Io / Serialize | Io | Session |
| Store Validation / Session Busy | Message | Session |
| Export / Trust | Io | Export / Trust |

用户可见 persist Display 仍是 `not found: {id}`，不叠 `session not found`。

## 2. 叶错误

- Clipboard：Io / Unsupported / Decode。Driver：Io → `io:`，Unsupported → `unsupported:`，Decode → `invalid input:`。
- Image：Io / Decode / Empty / Limit（crate 私有；进 Driver 仍走既有 Io 路径若有）。
- MCP：Connect / Call / Config / Timeout。装配口仍返回 `Option`；失败只进 diagnostics。
- Bridge tokenize：`AiBridgeError::Io(std::io::Error)`；reqwest 失败 `io::Error::other`。

## 3. 验证

复用 c2270 单测 + `just qa`。不新开 `.feature`。`skip_specs_landing: true`。
