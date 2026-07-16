# Design — c1140-add-package-tui-thinking-level-border

## 三层流水线（已拍板）

```text
packages/xylitol-tui          agent_demo                 src/app/tui
─────────────────────         ──────────────             ────────────
ThinkingBorderLevel           cycle 键/plate             （c1150）
Palette → border rgb/paint →  set_border_color           map domain level
Editor::set_border_color（既有）  harness 断言变色         + footer 回退
```

本变更 **只做左两列**。产品映射与 busy 策略留给 c1150；模型侧声明哪些 level 可用留给 c1145（包色阶已含 xhigh/max，与 pi 一致）。

## 包 API（最小）

| 面 | 行为 |
|---|---|
| `ThinkingBorderLevel` | `Off…Max`（含 xhigh/max，对齐 pi）；`as_str` / `parse` / `cycle_next` |
| `Palette::thinking_border_rgb(level)` | 返回边框色（各 level **可区分**） |
| `Palette::thinking_border_paint(level)` | `Box<dyn Fn(&str)->String>` 真彩包装 |
| Editor | **复用** `set_border_color`（ed08）；可选薄 helper `apply_thinking_border(&mut Editor, &Palette, level)` 仅调用 set_border_color |

**MUST NOT**：包依赖 `xylitol` / `domain::ThinkingLevel`。

## 色映射（对齐 pi）

来源：`../pi/packages/coding-agent/src/modes/interactive/theme/{dark,light}.json`
（`thinkingOff` … `thinkingMax`；命名色已按 theme vars 解析为 hex）。

**Dark**

| Level | hex | 备注 |
|---|---|---|
| off | `#505050` | darkGray |
| minimal | `#6e6e6e` | |
| low | `#5f87af` | |
| medium | `#81a2be` | |
| high | `#b294bb` | |
| xhigh | `#d183e8` | |
| max | `#ff5fff` | |

**Light**

| Level | hex | 备注 |
|---|---|---|
| off | `#b0b0b0` | lightGray |
| minimal | `#767676` | |
| low | `#547da7` | blue |
| medium | `#5a8080` | teal |
| high | `#875f87` | |
| xhigh | `#8b008b` | |
| max | `#af005f` | |

包内 `ThinkingBorderLevel` 覆盖上述 7 档；**MUST NOT** 复用 muted/accent/warning 等通用 token 冒充强弱。
测：相邻 level 色不等；dark/light 与上表 hex 一致。
## agent_demo

- **主入口**：`Shift+Tab`（对齐 pi `app.thinking.cycle`）
- 辅助入口（demo 可留）：Command plate `thinking-level`、`/thinking-level` slash
- cycle：`Off → … → Max → Off`（7 档）
- 应用：非 bash 模式时 `set_border_color(thinking_paint)`；**bash 模式仍用 success 边框**（thinking 暂存，退 bash 后恢复）
- harness：cycle 后边框 ANSI 含目标色分量，或经 test hook 读当前 level；含 `Shift+Tab` 键路径
## 与后续

- c1145：扩展/校验模型声明的 level 名；包可后加变体或 string fallback
- c1150：产品 host 把 `ThinkingLevel` → `ThinkingBorderLevel` + footer 回退；产品绑定 **`Shift+Tab`**（**不**加 `/thinking-level` 产品 slash，除非另钉）
