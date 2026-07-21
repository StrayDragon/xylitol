---
version: "alpha"
name: "glyphs"
description: "Configurable unicode/ascii glyph sets — no runtime font probing."
tokens_from: "../DESIGN.md"
components:
  user-glyph:
    textColor: "{colors.user}"
---

# Glyphs

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> **c475**：产品注入字符串；默认 `unicode`；可用 settings/env 切 `ascii`。
> 工具行 **不再** 使用 `⚙`/`*` 前缀 — 见 [`expandable.md`](./expandable.md) 工具设计语言（`Read path:12-40`）。

短前缀 glyph（用户 / 折叠 / 系统）由**应用面配置**选择。

| 配置档 | 用户 | 折叠 | 说明 |
|---|---|---|---|
| `unicode`（**产品默认**） | `❯` | `▶`/`▼` | 好看；依赖用户终端字体 |
| `ascii` | `>` | `>`/`v` | 最大兼容；复制也干净 |

## MUST

1. **MUST NOT** 做运行时字体/emoji 能力探测。
2. 用户显式配置（settings / 环境 / 启动项）切换档位；缺省 `unicode`。
3. 包内组件 **MUST NOT** 硬编码产品 glyph；由本面注入字符串或闭包。
4. busy spinner 用 braille 帧（与 demo 一致）；**不算** glyph 档切换范围。
5. 工具主视线 **MUST NOT** 使用齿轮等装饰 glyph；工具名 + 路径/位置即 chrome。

缺字体出现方块时：换 `ascii` 档或装字体——产品不自动猜。
