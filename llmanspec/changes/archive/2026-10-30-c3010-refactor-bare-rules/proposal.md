---
depends_on: []
title: 裸规则转场景与覆盖源固定
needs_specs_change: true
branch: sdd/c3010-refactor-bare-rules
base_branch: main
base_sha: 5a8df022287955876b2f9d80cd60b5c0a1caa348
---

# 裸规则转场景与覆盖源固定

## Why

- specs 中仍有 46 条规则只有规则句、没有嵌套 `场景:`，review 以 `unbound` 计数持续提示。
- 其中多数行为可程序化表达；少数是结构性不变量或已由单测/门禁覆盖，应显式固定覆盖源而不是留空。

## What Changes

- 将 28 条可程序化表达的裸规则补一个 `场景:`（不新增 BDD binding，不改运行时代码）。
- 将 18 条结构性/已由单测覆盖的裸规则补“覆盖源”说明，保持裸规则。
- 行为不变，仅让规格的可验证形状更完整。

## Capabilities

- `agent-plan` / `agent-prompt` / `agent-runtime` / `agent-session` / `agent-session-store` / `agent-workflow`
- `app-tui-bridge` / `app-tui-commands` / `app-tui-fixed-zone`
- `infra-observability` / `protocol-app` / `runtime-config` / `server-core`

## Impact

- **行为不变**：只补场景与覆盖源说明，不删规则、不改规则句的行为约束。
- **review**：`unbound` 预计从 46 降到 18（仅保留结构性/单测覆盖项）。
- **可回退**：删除新增场景与覆盖源句子即可回到原形状。

## Further Notes

- 新增场景不挂 `#[scenario]` binding；既有 BDD 绑定不变。
- 已声明“由 … 单测覆盖 / 不得单独扩 BDD step”的规则保持裸规则，只补覆盖源。
