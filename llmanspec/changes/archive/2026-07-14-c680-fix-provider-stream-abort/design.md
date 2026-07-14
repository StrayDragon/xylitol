# Design — c680 provider stream abort

## Gap

| 路径 | 现状 | 缺口 |
|---|---|---|
| 工具 / `!` bash | cancel → kill | 无（c660） |
| TUI Esc | idle + suppress Xy | 不关 HTTP（c670） |
| ReAct chunk 环 | `while next().await` | **无 cancel 竞态** |
| `XyModel` / adapters | `bytes_stream` / create_stream | 无显式 abort；依赖 Drop |
| reqwest | drop future/body → cancel | **底层支持** |

## 决策

1. **在 ReAct 修，不改 trait（第一拍）**：所有 client 经 `Driver::abort` → run token；chunk/`call_with_retry` 上 `select!` + drop stream → HTTP 关。
2. **证据分三层**：TCP drop PoC → agent 停 poll → BDD via `InProcessDriver`。
3. **不**在各应用面重复 abort HTTP 逻辑。

## Drop 语义

seanmonstar/reqwest：drop in-flight future 即 cancel。Anthropic/Responses 持有 `Response`→`bytes_stream`；drop `XyStream` 应导致对端 write 失败。Completions（async-openai）同理优先靠 drop；若证据失败再专项修。

## 失败模式

- 已生成、RST 前到达服务端的 token：厂商可能仍计费——只能尽早关连接。
- HTTP/2 大上传 body 历史坑与「小 JSON + 读 SSE」无关。

## 真机证据（2026-07-14）

llama.cpp server：Esc abort 后出现 `http client error: Connection handling canceled`，随后 `cancel task` / `slot release`。与 PoC「drop → 对端停写」一致。

## 测试命令

```bash
cargo test -p xylitol --test provider_http_stream_abort
cargo test -p xylitol abort_mid_stream_stops_polling_model_chunks
cargo test -p xylitol --test bdd test_agent_abort_mid_stream -- --test-threads=1
```
