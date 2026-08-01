---
depends_on: []
branch: sdd/c1780-update-tui-busy-instant-lists
base_sha: fd765a6b2b1320a60ca3a413569deb2f4630c3dc
checkpointed: true
checkpoint_sha: fd765a6b2b1320a60ca3a413569deb2f4630c3dc
---

# busy 即时列表闸（模型 / 主题 / Resume 只读）

> Roadmap：[docs/roadmaps/键位与命令发现.md](../../../docs/roadmaps/键位与命令发现.md) **M0**。
> 文案已拍板：**A** `agent busy — finish turn or Esc abort before switching session`

## Why

agent busy 时多数「即时列表」被 `BusySlashPolicy` 整命令 Reject，用户无法在对话进行中看模型列表、切主题、浏览会话列表——与「列表应与 scroll / Provider 飞线无关」的体验目标冲突。Resume 需要 **可浏览、不可 switch**：switch 须等 turn 结束或 Esc 中止，并用 **滚动提示**说明。

## 已拍板

| 项 | 决定 |
|---|---|
| `/model` 无参开列表 | busy **Allow**；确认仍走既有 NextTurn / 下轮预告 |
| `/model <id>` | 保持 Allow |
| `/theme` | busy **Allow**（开列表与有参应用） |
| `/mcp` | 保持 Allow |
| `/session-resume` 打开 / 浏览 / 搜索 | busy **Allow** |
| Resume **Enter switch**（busy） | **Reject**：MUST NOT `SwitchSession`；MUST 尾随滚动提示 **A** |
| `/reload` `/trust` / tree / fork / new / clone / import | 保持 Reject |
| Command Plate / `/hotkeys` | **不做**（本 change 范围外） |

## What Changes

- 改写 `slash_allowances`：`OpenModels` / `Theme` / `OpenSessionResume` → Allow
- Resume 面板：busy 下确认 switch → 滚动提示 A，不切会话
- 更新 live specs：`app-tui-commands`（atm1/10/15/16 等）、`app-tui-input`（ati21 等）
- harness：busy 开列表；busy switch 拒 + 文案 A；既有 busy reject 不回归
- 同步 `design/keybindings.md` 一句（若有 busy 描述）

## Capabilities

- `app-tui-commands`（atm1、atm10、atm15、atm16；必要时 atm17 交叉一句）
- `app-tui-input`（ati21；ati29 若需注明 busy switch）

## 测试缝（apply 前已对齐）

复用既有产品 TUI harness / `commands::slash_allowances` 边界（与 atm16-unit 同路），**不**新发明脱离 harness 的 CLI 缝：

| 缝 | 断言 |
|---|---|
| `slash_allowances` | OpenModels / Theme{..} / OpenSessionResume = Allow；Reload/Trust/… 仍 Reject |
| harness busy + `/model` | Models 槽打开；MUST NOT 「unavailable while busy」拒开 |
| harness busy + `/theme` | Themes 槽可开（或有参可应用） |
| harness busy + `/session-resume` | Resume 面板打开可浏览 |
| harness busy + Resume Enter | MUST NOT switch；`UiEntry::ScrollNotice` 含文案 A |
| harness busy + `/reload` | 仍 `agent busy — /reload refused` |

## Impact

- 忙碌中可改 chrome / 浏览会话；误 switch 有明确滚动提示
- 风险：busy 开 overlay 时 Esc 先关槽不 abort（既有规则）；须 harness 钉清

## Out of scope

- `/hotkeys`、Command Plate、可视化键盘、改默认键位表
- Resume busy 下 rename/delete（本波：若 Enter 会触发写操作，同样拒 + 提示或保持子态不可确认——实现时与 switch 同闸优先）
