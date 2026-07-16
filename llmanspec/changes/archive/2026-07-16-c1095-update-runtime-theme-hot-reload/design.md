# Design — c1095-update-runtime-theme-hot-reload

## Decision

### 1. 内建优先

本波 **只保证** `dark` / `light` → `Palette::dark()` / `light()`。磁盘上的 `themes/*.json` 仅贡献**名称目录**（供未来 c1115）；自定义色值解析推迟，避免与 DESIGN token 闸分叉。

### 2. Trust + 发现

`discovered_theme_names(cwd, agent_dir, trusted)` 在 `app/core`：与 context 相同，untrusted 时 loader cwd = temp。返回排序去重的名字（含文件 stem）。

### 3. Apply 路径

```text
settings.theme / 调用方 name
  → resolve_builtin_palette(name)
  → LayoutTheme::from_palette
  → UiRoot::apply_layout_theme
  → invalidate / request render（host）
```

失败：返回 Err/诊断，**不**改 `UiRoot.theme`。

### 4. 与 XyReloadable

主题文件缓存刷新走 `DefaultResourceLoader` 的 `XyReloadable::reload`（c1100）；产品 apply 是独立的 Palette 缝。

## Trade-offs

- 不做完整 pi Theme JSON：范围可控，DESIGN SSOT 不被旁路。
- Host 默认仍 `product_dark`；reload 仅在显式调用 / 未来 `/reload` 时切换。
