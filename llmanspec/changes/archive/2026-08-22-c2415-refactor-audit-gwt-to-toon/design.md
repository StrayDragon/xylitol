# Design

## 映射规则

| .feature 元素 | toon 行 |
|---|---|
| `@req:X` | req_id 列 = X |
| `场景: english-id` | id 列 = english-id（保留） |
| `假如 …` / 缺省 | given 列（缺省空串） |
| `当 …` | when 列 |
| `那么 …` | then 列 |
| 终列 | `false` |

- 本批五文件均无 `并且` 链与占位符；后续批次如遇 `并且`，并入 when/then 以 `；` 连接。
- TOON 引号：含空格/逗号/冒号/方括号的单元格加双引号（与既有行风格一致）。
- 与既有 toon 行同 id 冲突检查：本批无冲突（observability 既有 9 行 id 不重）。

## 已知特例

- `infra-observability.feature` 头部 `功能: infra-provider-trace` 名称漂移——随文件删除消解，不另立条款。
- `ipt2 release-off-by-default` 与既有 `gate-off-no-cost` 主题相近但角度不同（文件零增长 vs API 空操作），两行并存，压缩裁决留给后续 specs-compact 票。
