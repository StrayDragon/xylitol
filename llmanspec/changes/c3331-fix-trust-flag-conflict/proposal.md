---
depends_on: []
title: "Fix trust flag conflict rejection"
---

# --trust / --no-trust 同传显式报错（F8 拆出项）

## Why

c3305（SDK/CLI 词表收敛）复核确认的 F8 残留：`--trust` 与 `--no-trust` 同传时 `src/cli.rs` 的 match 首臂 `(true, _) => Some(true)` 静默取信任——用户输入矛盾时选择了**放松门禁**的方向，与 trust「宁严勿松」取向相悖。args 定义无 `conflicts_with`。该行为变更当时裁定从 c3305 拆出（见其归档 design.md §1 边界），本 change 承接。

## What Changes

- 同一命令同传 `--trust` 与 `--no-trust`：解析层显式报错、非零退出，不进入 trust 门禁流程。
- 跨层组合（root 级 + 子命令级各传一个，经 `intent.rs` OR 合并后双真）同样报错。

## Capabilities

- cli-entry（CLI 入参硬失败契约；与既有 no-silent-fallback 同族）。

## Impact

- `src/cli/args.rs`（3 个 flag 组）、`src/cli.rs` / `src/cli/intent.rs`（合并点校验）；clap 报错文案 + 合并点错误信息。BDD 补场景。

## Further Notes

- 拆出来源：llmanspec/changes/archive/2026-11-13-c3305-refactor-sdk-cli-state-vocabulary/design.md §1（F8 拆出裁定）。
