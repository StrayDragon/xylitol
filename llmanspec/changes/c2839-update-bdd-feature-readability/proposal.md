---
depends_on: []
---

## Why

BDD feature 文件是产品行为的**人类主要阅读入口**（r1813 契约：人类参考 features 了解行为）。调研显示：
- 格式噪声实测仅 3 处 `@req` 块间无空行（早期口径过宽，已精修）；
- 「同步序重复」72 组中真重复组（如 before-hook 拒绝 ×4）**跨 capability 不可物理合并**（spec 单轨：每 capability 独立 feature 自证）——合并价值 = 候选清单 + 归属说明，而非去重；
- 大文件（agent-tools 585 行/86 场景、server-core 504/67 等 8 个 >300 行）缺文件级导航。

本 change 在**不动场景语义/绑定契约**前提下做可读性治理。

## What Changes

- 修复 3 处 `@req` 块间无空行（格式规范化）
- 8 个 >300 行 capability feature 头部加「语义索引注释块」（按规则标题归组，人类 Ctrl-F 导航）
- 产出「跨 capability 重复场景候选清单」进 research/（判定真伪 + 归属建议，不做机械去重）

## Out of Scope

- 不合并/删除场景（跨 cap 不可物理合并；语义去重留清单指导后续）
- 不改步骤绑定与 r1913/r37 等既有结构
- 场景 id 中文语义注（边际价值低——步骤文本已中文；若需再加）
