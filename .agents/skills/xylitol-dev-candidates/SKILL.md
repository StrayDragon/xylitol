---
name: xylitol-dev-candidates
description: >-
  xylitol Host 监听器与线协议周边依赖候选：salvo（现行明文 loopback HTTP/WS）、
  jsonrpsee（JSON-RPC 文本已退役）、axum（未在锁内）、Scalar/salvo-oapi（调试文档已拆）。
  已退役本地词表 / RemoteCount：tiktoken-rs、HF tokenizers、gigatoken、
  Anthropic count_tokens / OpenAI input_tokens HTTP。
  在考虑换 HTTP 框架、复活 JSON 文本通道、加回 OpenAPI/Scalar、
  或再引入本地词表 / RemoteCount 时使用。
---

# xylitol 开发候选（Host 监听器 / 线协议周边）

**边界**：何时再拿某依赖出来。一次性迁移步骤不进本 skill。现行产品真源：`POST /rpc` + `WS /rpc` 的 v3 fory 帧；`GET /healthz` 同端口。计量现行：Api usage → Heuristic（UTF-8 字节 `/3`）。

## 现行

| 件 | 角色 |
|---|---|
| salvo（`default-features=false`，`server` / `server-handle` / `http1` / `websocket` / `test`） | 明文 loopback Host：healthz + `/rpc` HTTP/WS |
| fory（xlang） | 产品线协议编码 |
| 内部 `dispatch_raw` | Host 方法表仍吃 JSON 形载荷；**不是**对外 JSON-RPC 通道 |

## 已退役 / 未引入 — 何时再拿出来

| 件 | 状态 | 再拿出来的触发 |
|---|---|---|
| jsonrpsee | 已退役 | 只有当产品再次承诺 **JSON-RPC 2.0 文本** 为 `/rpc` 业务通道（与 fory 并列或替换）。调试手搓 JSON 不够格。 |
| salvo-oapi + Scalar（`/openapi.json` / `/docs`） | 已拆 | 人手调试文档成为产品承诺，或外部集成需要生成客户端。方法表 SSOT 仍是 `protocol/wire/registry`，不要为文档另开第二套清单。 |
| axum | 未在锁内 | salvo 的 feature 剪枝仍盖不住实际用到的 HTTP/WS 面，或要换运行时（tower 生态、超体中间件）。**不要**为「看起来更主流」引入。 |
| 标准库独立 healthz 进程 | 未做 | 只有当同端口 salvo 成为不可接受的装配成本，且 attach 预检仍要 pid/version/starting-503。healthz 不能变成第二个产品 HTTP 面。 |
| tiktoken-rs | 已退役 | 产品再次承诺本地 OpenAI-family encode，且要与 openai/tiktoken 模型→encoding 表对齐。不要为「看起来更准」单独加回。 |
| HuggingFace `tokenizers` crate | 已退役 | 需要 `tokenizer.json` 精确计数。对照消费点曾是 `tokenize::encode_count_at_path`。调研：`llmanspec/delayed-changes/models/c1530-update-local-tokenizer-gigatoken/research/` |
| Gigatoken | 未引入（曾评估） | 仅当重新交付 LocalTokenizer **且** HF encode 热路径成本不可接受。可行性见同目录 `gigatoken-rust-feasibility.md`。不要为宣称吞吐单独引入。 |
| Anthropic `POST /v1/messages/count_tokens` · OpenAI `POST /v1/responses/input_tokens` | 已退役产品路径（RemoteCount） | 产品再次承诺独立 HTTP 计数档，且 Api `usage` 不够用。实现曾在 `provider/remote_count.rs`。 |

## 硬约束

1. 产品 `/rpc` **MUST** 保持单一载体（当前 fory-v3）。加回 JSON 文本 = 行为合约变更，走 propose。
2. 换框架 **MUST NOT** 改变方法表、租约、幂等、订阅 seq。
3. 候选评估先看 Cargo feature 实际启用面与 `just qa`，再谈替换。
4. 复活本地词表 / RemoteCount HTTP = 行为合约变更，走 propose。旧 YAML `tokenizers:` / `models.*.tokenizer` / `token_estimate.local_tokenizer` 静默忽略，**不加**加载 warn。
