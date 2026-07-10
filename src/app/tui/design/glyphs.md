---
version: "alpha"
name: "glyphs"
description: "Configurable unicode/ascii glyph sets — no runtime font probing."
tokens_from: "../DESIGN.md"
components:
  user-glyph:
    textColor: "{colors.user}"
  tool-glyph:
    textColor: "{colors.tool}"
---

# Glyphs

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。

短前缀 glyph（用户 / 工具 / 状态等）由**应用面配置**选择。

| 配置档 | 用户 | 工具 | 说明 |
|---|---|---|---|
| `unicode`（默认意向） | `❯` | `⚙` | 好看；依赖用户终端字体 |
| `ascii` | `>` | `*` | 最大兼容；复制也干净 |

## MUST

1. **MUST NOT** 做运行时字体/emoji 能力探测。
2. 用户显式配置（settings / 环境 / 启动项）切换档位。
3. 包内组件 **MUST NOT** 硬编码产品 glyph；由本面注入字符串或闭包。

缺字体出现方块时：换 `ascii` 档或装字体——产品不自动猜。
