# Design — c545-update-package-tui-editor-sources

## 扩展点

```
Editor.set_completion_sources([Slash, AtPath, /* optional Dollar, Caret */])
probe 顺序 = 注册顺序；未注册 = 零成本
```

本变更只保证：**可注册空实现/测试桩** + **窄宽不溢出**；不交付产品 `$`/`^` 语义。

## 窄宽

popup 渲染宽 ≤ editor 内容宽；超长项截断策略对齐既有 SelectList。
