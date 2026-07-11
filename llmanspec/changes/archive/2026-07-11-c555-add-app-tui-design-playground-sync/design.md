# Design — c555-add-app-tui-design-playground-sync

## 角色

```
DESIGN.md tokens ──sync_tokens.py──► tokens.css / tokens.js
design/*.md MUST ──示意──► playground 槽（人类审）
包实现 + 测试 ──真值──► 运行时（Agent 默认读 md，不读 HTML）
```

## Markdown 槽

示意：无 `#` 前缀；链接 `text (url)`；粗体/斜体靠样式 + 可选（加粗）/（斜体）文案，无可见 `**`/`*`。
