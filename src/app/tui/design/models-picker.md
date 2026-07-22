---
version: "alpha"
name: "models-picker"
description: "/model selects model + xylitol thinking levels — wide: all levels; narrow: cycle; no-thinking rows."
tokens_from: "../DESIGN.md"
components:
  models-title:
    textColor: "{colors.muted}"
  models-selected:
    textColor: "{colors.on-surface}"
  models-match:
    textColor: "{colors.accent}"
  models-hint:
    textColor: "{colors.muted}"
  models-level:
    textColor: "{colors.muted}"
  models-level-active:
    textColor: "{colors.on-surface}"
---

# Models picker（`/model` + thinking）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 键位：[`keybindings.md`](./keybindings.md)。Pending：[`pending-runtime.md`](./pending-runtime.md)。
> 静图：[`playground/`](./playground/)「Models」槽。

## 产品意图

换模型与思考等级走**同一入口**（`/model`）。等级只用 **xylitol 档名**；厂商差异关在 `thinking_level_map`。

- **禁止**全局 Shift+Tab cycle thinking。
- **↑↓** 选模型；**←→** 在焦点模型的支持集上选等级；**Shift+Tab** 亦可 cycle（与 ←→ 同槽，不另开全局绑定）。
- **宽度够**时：焦点行（或每行，见下）**铺开该模型全部** xylitol 档，当前暂定档高亮。
- **宽度不够**：退化为单档标签 + ←→ / Shift+Tab cycle（不挤爆一行）。
- **不支持思考**的模型：无等级可选；行上明确「不可调」。

## xylitol 等级体系（产品面）

| 原则 | MUST |
|---|---|
| 用户可见名 | 仅 xylitol `ThinkingLevel`（`off` … `max`）；**MUST NOT** 显示 provider 专有名 |
| 支持集 | 每模型子集；`thinking: false` / 仅 off → **无思考可调** |
| Provider | 仅边界 map；UI 不泄漏 |
| 默认 | 可调模型：支持集 **最高档**；不可调：固定 `off`（或不展示等级语义） |
| 全序 | `off < minimal < low < medium < high < xhigh < max`；最高 = 支持集最大元 |

## 布局：宽 / 窄 / 无思考

记 `levels(m)` = 模型 `m` 的 xylitol 支持集（可调 ⟺ `levels(m)` 含至少一档且非「仅 off 占位不可调」——产品定义：**仅 `off` 或空 = 不支持思考设置**）。

### 宽度判定（实现 MUST 可测）

在 picker 内容宽 `W` 下，对**焦点模型** `m`：

- 估算「整行铺开」所需宽：`id列 + gutter + join(levels(m), 空格或间隙) + 高亮开销`（ANSI 宽度，与包宽工具一致）。
- **够宽**（`needed ≤ W` 且可调）：**wide** 模式——展示 `levels(m)` **全部**档位，暂定档 **reverse / 高亮**（选中行内仍遵守「不叠 kind 前景」：用 `.rev` 只包当前档 token，或整行 rev 时档位用 `[]` / `*` 标当前）。
- **不够宽**或窄终端：**narrow** 模式——只显示当前暂定档一个标签；←→ / Shift+Tab 仍 cycle。
- 换焦点模型时重新判定（支持集长度不同）。

静图定形：**焦点行**在 wide 下铺开该模型全部档；**非焦点行**只显示一个预览档（默认可为该模型最高档，或 `—` 若不可调），避免多行都铺满导致噪声。

### 不支持思考（MUST）

| | |
|---|---|
| 判定 | 支持集为空，或仅 `off` 且模型声明无思考（`thinking: false`） |
| 行展示 | 等级区为 muted `—`（或 `no thinking`，宜短；静图用 `—`） |
| ←→ / Shift+Tab | **MUST NOT** 改变任何等级；**MUST NOT** 假高亮 |
| Enter | 只提交模型；thinking 固定 `off` |
| 默认 | 无「最高档」选择问题 |

## MUST

1. **`/model`（无参）** 打开选择器（替换 editor 槽）；**MUST NOT** 居中 overlay。
2. **`/model <精确 id>`** 直设模型；可调 → thinking = **最高档**；不可调 → `off`；未知 id 短错误。
3. **↑↓**：移动焦点；进入可调模型时暂定等级 =（若为当前模型且当前 thinking 仍在支持集）当前 thinking，否则 **最高档**；进入不可调模型时暂定 = `off`。
4. **←→**（焦点可调）：在 `levels(焦点)` 上移动暂定档（左更低、右更高，按全序）；到端不再绕或绕回——静图/实现取 **循环**（与 Shift+Tab 一致，避免卡死）。
5. **Shift+Tab**（仅 picker 开、焦点可调）：与 ←→ 同为 cycle 暂定档（单向即可）；**MUST NOT** 全局绑定。
6. **Enter**：提交 `(焦点模型, 暂定等级)`；不可调则 level=`off`。
7. **Esc**：关槽，不改模型/thinking。
8. **Fuzzy**：过滤模型列表；规则同前。
9. **Busy**：开列表默认 idle-only；提交后 pending 见 pending-runtime。
10. **Footer**：xylitol 档名；不可调模型可不显示 thinking 字段或显示 `thinking off`——静图：仍 `· thinking off` 以保持列稳定，或省略；**定形为不可调时 footer 省略 thinking 段**（`cwd · model` only）以免假装可调。实现与 footer.md 对齐：无可调思考时 **省略** `· thinking …`。

## 形状（固定）

### Wide（焦点可调，档全看得见）

```
models · ←→ level · shift+tab · enter
› ornith-fast *     off  low  medium [high] max
  ornith-think                          max
  other/gpt-mini                          —
```

`[high]` = 暂定档（高亮）；非焦点行只显示预览一档或 `—`。

### Narrow（不够宽）

```
models · ←→/shift+tab level · enter
› ornith-fast *                      high
  ornith-think                        max
  other/gpt-mini                        —
```

### 焦点在无思考模型

```
models · enter
  ornith-fast                         high
  ornith-think                         max
› other/gpt-mini                         —
```

←→ / Shift+Tab 无等级反馈（可不 beep；**MUST NOT** 改状态）。

选中行整行 `.rev` 时，档位高亮用 `[token]` / 空格分隔，**MUST NOT** 在 rev 行内再叠 `fg-user` 等 kind 色。

## 键位（仅槽内）

| 键 | 行为 |
|---|---|
| 字符 / 退格 | fuzzy |
| **↑↓** | 焦点模型 |
| **←→** | 焦点可调时选档（循环） |
| **Shift+Tab** | 焦点可调时 cycle 档 |
| Enter | 提交 |
| Esc | 取消 |

**全局**：**MUST NOT** Shift+Tab → thinking。

## 不做

- 全局 thinking cycle / `/thinking-level`。
- UI 暴露 provider 思考枚举。
- 不可调模型上伪造多档。
- `/models`；Settings/Plate；Scope（另切片）。

## 验收指针

- playground：`wide` / `narrow` / `no-thinking` / `filter` / `empty`。
- 够宽见全部档 + ←→；不够宽单档；无思考为 `—` 且 ←→ 无改。
- 默认最高档；精确 id 直设同规则。
