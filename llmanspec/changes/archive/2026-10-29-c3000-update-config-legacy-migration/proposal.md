---
depends_on: []
title: 配置迁移：拒绝 provider preset 旧写法并给出迁移指引
needs_specs_change: true
branch: sdd/c3000-update-config-legacy-migration
base_branch: main
base_sha: 94a1130aac12c77a96ff66f5811b3a2deff9f588
---

# 配置迁移：拒绝 provider preset 旧写法并给出迁移指引

## Why

- #35 的方向之一是「配置清理：提供配置迁移 / 废弃键处理 / 最小配置校验，避免预设回滚后留下无效字段或旧写法」。
- c2995 的 provider preset 已回滚（#20 / #36），但旧配置里的 `models.<alias>.preset` 目前会被 serde **静默忽略**（`XyModelEntryConfig` 未拒绝未知字段），用户会误以为 preset 仍生效，实际拿到的是缺字段的模型条目。
- 顶层 `provider_presets` 同样会被静默忽略。两类旧写法都需要在加载期硬失败并给出可执行的迁移指引。

## What Changes

- `xylitol-official-plugin-config` 加载器新增旧写法检测：
  - `models.models.<alias>.preset` → 拒绝，提示改为显式 `provider` / `model` / `base_url` / `api` / `compat`（或经网关）。
  - 顶层 `provider_presets` → 拒绝，提示按模型条目显式配置。
  - 错误文案 须点名旧键、迁移方向与 `configs/example.yaml`，不得只给 serde 泛化错误。
- 规格：
  - `runtime-config` 新增 `legacy-provider-preset-migration` 规则与场景。
- 文档：
  - README 增加一段迁移说明（preset 已回滚；显式字段 / 网关二选一）。
- 测试：
  - 配置 crate 单测覆盖 model preset、顶层 provider_presets、正常配置不受影响。
  - BDD 场景覆盖旧键拒绝与错误文案含迁移指引。

## Capabilities

- `runtime-config`（旧 provider preset 键迁移）

## Impact

- **行为变化**：带旧 `preset` / `provider_presets` 的配置从「静默忽略」变为「加载期硬失败」。
- **迁移成本**：用户按错误提示把 preset 展开为显式字段，或改走网关。
- **不变**：正常配置、模型解析、provider 插件装配与 ReAct 主线不变。

## Non-Goals

- 不恢复 preset 清单或内置供应商预设。
- 不实现供应商插件元数据注册中心 / IPC / rutis 非 Rust 插件。
- 不集成具体网关（aimux 等）。
- 不做配置文件自动改写。
