---
change_id: c626-add-app-tui-playground-lint
title: "静图 L1 lint + L2 fixture：自动挡 DESIGN 漂移"
status: full
priority: 626
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: C
---

# c626-add-app-tui-playground-lint

## Why

色板已有 `check-tui-tokens`，但形状/文案/选中对比度仍靠人眼。需要 **可进 `just qa` 的机械闸**，把已知漂移类变成失败规则，并引入 L2 fixture 格式。

## Purpose

1. **L1**：`scripts/check_tui_design_playground.py`（经 `check-scripts` 进 `just qa`）。
2. **L2**：`design/fixtures/*.yaml` + playground `data-design-fixture`；至少 `session-tree.filter` 与 `models.open`。
3. 扩 `app-tui-design-playground`（adp6–adp8）。

## What Changes

1. 新建 check 脚本；规则见 c625 `design.md` L1/L2（`cNNN` 噪音仅扫 Next-wave 槽与 fixture 源，避免误伤历史壳标题）。
2. fixtures + HTML `data-design-fixture`。
3. 文档指针：`design/AGENTS.md`、playground README。

## Capabilities

- `app-tui-design-playground`（modify）

## Design SSOT

- `llmanspec/changes/c625-update-app-tui-design-next-wave/design.md`
- `src/app/tui/design/session-tree.md`（选中 reverse）
- `src/app/tui/design/models-picker.md`

## Out of scope

- 截图回归；与 Rust 逐字符一致

## Ethics

- risk_level: low
- required_evidence: 故意违规失败；干净树绿
