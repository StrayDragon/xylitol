---
depends_on: []
branch: sdd/c2345-update-chrome-run-binding
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: true
checkpoint_sha: f8a1bbd8909d02271a161407a004a39613bc3243
---

# chrome 合约同句：attach run 绑定语义

P0 已落地 run 绑定（`prompt` 冻结模型、busy 切模 footer 立即反映 selected、无 NextTurn chrome），但 `app-tui-chrome` 合约仍保留旧下轮预告条款（atc19 MUST 显示 cue），atc7/atc11 携带过期交叉引用，词汇表词条标注「随面退役一并清词」而未清。本变更把合约与文档同句到现行拓扑——纯合约/文档收口，零代码行为变化。

## Why

- attach 是唯一产品 TUI 拓扑；`XyDriver::active_turn()` 默认 `None` 且 Remote 未覆写，产品面从不渲染 cue——atc19 的 MUST 在现行产品中不可满足，属合约与事实漂移。
- 依据 `_TUI_MIGRATED_TODO.md` P0 决议：「词表词条保留至 P1 随 spec 一并退役」。

## What Changes

- **app-tui-chrome**：退役 atc19（删除 requirement 及 `.feature` 三个场景）；atc7 去交叉引用；atc11 busy 条款改写为 run 绑定语义并删除旧语义场景 `busy-footer-stays-active`；atc25 措辞去概念引用。
- **docs/architecture**：词汇表删「下轮预告」词条、弃用表标注退役、落点/原则行清理；`库与多客户端.md` 对拍收口表述。
- **不改代码**：cue 机械码（`status_next_turn_cue_text` 等）保留——harness 合成驱动与 embed-InProcess 组合仍合法使用。

## Capabilities

- `app-tui-chrome`（spec 措辞同句）。

## Impact

- 无行为变更、无 wire 变更；BDD 场景均为无绑定文档 GWT，删除不影响 harness。
