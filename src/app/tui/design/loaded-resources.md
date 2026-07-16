---
version: "alpha"
name: "loaded-resources"
description: "Minimal Codex-style bordered startup card; wrap skills/MCP; no ellipsis; no ASCII logo."
tokens_from: "../DESIGN.md"
components:
  loaded-card-border:
    textColor: "{colors.muted}"
  loaded-title:
    textColor: "{colors.on-surface}"
  loaded-skills-label:
    textColor: "{colors.skill-ref}"
  loaded-mcp-label:
    textColor: "{colors.success}"
---

# Loaded resources（启动 header）

> Token：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。呈现：c1135 polish（简约 Codex 卡片）。与 [`../PI_DELTAS.md`](../PI_DELTAS.md) **A10** 正交。

## MUST

1. 位置：scrollback **上方**独立槽。
2. **卡片**：`╭─╮ / │ │ / ╰─╯` 边框；内容为 `>_ xylitol (version)` + `model` / `directory` / `skills(N)` / `mcp`。
3. **Skills / MCP**：标签语义色；值用 ` · ` 分隔并在卡内换行 **全量** 展示；**MUST NOT** `...`。
4. **MUST NOT** 展示「木糖醇」中文标签、化学式标题行、或 ASCII logo 艺术字。
5. 无 skills/MCP 时仍保留卡片（title + model + directory）。
6. **MUST NOT** 列出 prompt templates / 密钥。
7. 数据经 `Driver::loaded_resources_snapshot` + UiRoot `cwd`/`model`；启动与 `/reload` 成功后刷新资源行。
8. 卡片每行 **exact width** pad（降低差分 resize 残影）。

## MUST NOT

- 常驻键墙 / debug strip
- 把本槽当 skill **调用**验收（A10）
