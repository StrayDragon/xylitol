# Design — c1060 OpenAI RemoteCount

## 端点

`POST {base_url}/v1/responses/input_tokens`

Body：与 Responses create 同形的 `model` + `input`（本包复用 `messages_to_responses_input`）。

Response：`{ "object": "response.input_tokens", "input_tokens": <u64> }`。

鉴权：`Authorization: Bearer <api_key>`（与现有 OpenAI adapter 一致）。

## 装配

```text
caller (async)
  → RemoteCounter::count_tokens(messages)   // Anthropic | OpenAI | Stub
  → EstimateOpts { allow_remote_count, remote_count_tokens: Some(n) }
  → estimate_context / estimate_context_tokens_with
  → provenance RemoteCount
```

失败 / 超时 / 关闭：不填 `remote_count_tokens`（或 count Err），链落到 LocalTokenizer → Heuristic；**MUST NOT** 标 Api。

Completions-only 兼容端：无 input_tokens 端点时 registry 不声明 RemoteCount；继续 LocalTokenizer。

## Completions vs Responses

本 change **只**接线 Responses `input_tokens`。Completions 路径仍用 tiktoken LocalTokenizer（Api usage 仍最高优先）。
