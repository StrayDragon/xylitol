---
depends_on: []
status: in-progress
branch: feat/c1470-nextturn-model-thinking
base_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
checkpointed: true
checkpoint_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
---

# c1470 — NextTurn 双态 chrome + `/model` 内 thinking

## Why

1. busy / in-flight 改模型或 thinking 时，用户须一眼看到 **生效中** vs **即将接替**，且成功路径无 System 墙。
2. 全局 Shift+Tab cycle thinking 是外漏设置，与 `/model` 两套心智。
3. Provider 思考档差异大：产品面只讲 **xylitol 等级**；厂商字段仅边界 map。
4. 今日 ReAct 整 run 握死 `Arc<XyModel>` + `generate_options`，无法 NextTurn 换模。

## What Changes

按 **依赖序**（tasks 同序）：

| 序 | 层 | 交付 |
|---|---|---|
| 1 | agent-runtime | turn 边界重读 selected model + thinking（NextTurn）；idle/abort **收敛** |
| 2 | runtime-model-registry | 换模默认 **支持集最高档**（可调）；不可调 → `off` |
| 3 | app-tui `/model` | 列表内 ↑↓ 模型、←→/Shift+Tab 选档；wide 铺档 / narrow 单档 / 无思考 `—`；移除全局 thinking cycle |
| 4 | app-tui chrome | Status trail（lead 左、trail 右）；footer=active；成功无 System |
| 5 | design playground | 已定形闸门保持绿 |

产品真源：`docs/roadmaps/运行时即时设置.md` M0。
视觉 SSOT：`src/app/tui/design/{models-picker,pending-runtime,status,footer,keybindings}.md`。

## Capabilities

- `agent-runtime`（新 req：turn 边界刷新 model/thinking）
- `runtime-model-registry`（modify m10 默认最高档）
- `app-tui-commands`（modify atm1）
- `app-tui-input`（modify ati21 / ati36）
- `app-tui-host`（modify ath22）
- `app-tui-chrome`（modify atc2/atc7/atc11；新 trail req）
- `app-tui-design-playground`（modify adp9）
- `package-tui-agent-demo`（modify pad6：产品不再占用全局 Shift+Tab）

## Impact

- **破坏**：产品全局 `app.thinking.cycle` / idle·busy Shift+Tab cycle **废止**；demo 可保留 Shift+Tab 仅 demo。
- **兼容**：`thinking_level_map` 仍只服务请求组装；UI 禁止厂商档名。
- **文档闭环（兑现后）**：迁入 `docs/architecture/`，收缩 roadmap。

## Out of scope

- SlashMeta 命令表重构（可另 change）
- Scope `all|scoped`；能力覆盖盘 M1+
- Settings/Plate
