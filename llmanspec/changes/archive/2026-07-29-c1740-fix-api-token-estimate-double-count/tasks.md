# Tasks: c1740-fix-api-token-estimate-double-count

## 1. Specs

- [x] 1.1 修订 live `package-ai-bridge-accounting`：新增/收紧 Api trailing 范围（对齐 pi：仅 last-usage 之后）；`.feature` 可执行场景
- [x] 1.2 `llman sdd change start` 绑定 feature 分支

## 2. Implementation

- [x] 2.1 `estimate_context`：trailing 仅计 usage 锚点之后的消息；写入正确 `last_usage_index`
- [x] 2.2 单测：长历史 + 末条 usage → tokens ≈ usage（±trailing 仅新消息），不得 ≈ usage+heuristic(全部)
- [x] 2.3 既有 accounting / compaction 相关测试全绿

## 3. Gate

- [x] 3.1 `llman sdd validate c1740-fix-api-token-estimate-double-count --strict`
- [x] 3.2 相关 `cargo test` / BDD 门禁
