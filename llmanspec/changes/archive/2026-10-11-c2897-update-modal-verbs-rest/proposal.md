---
depends_on: []
branch: sdd/c2897-update-modal-verbs-rest
base_branch: main
base_sha: 6d6fb38a2d9e49d37ec3980d136f1e4811f10243
---

## Why

#10 后续批(接 c2896):上一批已清零 GWT 步骤行与 Top 8 密度文件,剩余 1458 处
MUST/SHALL 全部为规则描述行(38 个文件)——零 runner 匹配、零 tests/bdd 耦合,
风险低于已完成批次。随触清会持续数月且把措辞噪声混进未来行为变更 diff,与
「改动聚焦」冲突;验收条款「单调下降、不设清零死线」允许,本批一次清零。

## What Changes

- 全部 38 个 spec 文件的规则描述行(含 1 处 `# purpose` 头注释)清零 MUST/SHALL:
  MUST NOT → 不得、MUST/SHALL → 须、`**MUST NOT**` → `**不得**`(与 c2896 同款映射)
- 无 GWT 步骤行参与(上批已清零),无需同步 tests/bdd;行为合约不变
