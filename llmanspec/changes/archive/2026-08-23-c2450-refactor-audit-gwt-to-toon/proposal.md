---
depends_on: []
branch: sdd/c2450-refactor-audit-gwt-to-toon
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: true
checkpoint_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
---

# 未绑定审计 GWT 迁回 toon（batch 2：app-tui-input / host / chrome）

specs 精简票第二批：batch1（c2415）验证迁移模式后，本票一次性清掉三个纯未绑定大户——
`app-tui-input`（83）、`app-tui-host`（51）、`app-tui-chrome`（44），共 **178 个场景**。
它们以可执行语法承载审计陈述语义、从不进 harness；迁为 `spec.toon` 的 `feature:false`
文档行并删除对应 `.feature` 后，`.feature` 只剩真执行场景。

## Why

当前 782 个场景中 489 个（~62%）无 `#[scenario]` 绑定、从不执行，误导「以为在跑」。
三大 TUI 户合计占全部 unbound 的 36%；整文件零绑定，可整文件删除，迁移风险最低。

## What Changes

- 迁移并删除：`llmanspec/specs/app-tui-input/app-tui-input.feature`（83）、
  `app-tui-host/app-tui-host.feature`（51）、`app-tui-chrome/app-tui-chrome.feature`（44）。
- 每场景转为 toon `scenarios[]` 一行（终列 `false`），scenario id 保留英文原名。
- 零行为变化：这些场景本就不执行；静态一致性闸（check_bdd_steps）对 bound 集合的判定不受影响。

## 非目标

混合文件（agent-session-store / runtime-model-registry / app-tui-bridge /
package-ai-bridge-accounting / package-ai-bridge / agent-prompt / agent-runtime /
domain-compaction 等）中未绑定行留待后续批次；场景内容改写。

## Impact

toon scenarios 计数：input 3→86、host 14→65、chrome 6→50；3 个 `.feature` 删除；
harness 口径与实际执行的偏差收窄约 178 行。
