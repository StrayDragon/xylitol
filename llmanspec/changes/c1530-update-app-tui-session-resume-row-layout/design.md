# Design: c1530-update-app-tui-session-resume-row-layout

## Decision

Resume 会话行在「宽预览」与「排障 UUID」之间采用 **默认隐藏 + 快捷键展开**：

| 态 | 列 |
|---|---|
| 默认 | 预览（软顶 ≈ 终端宽 60%）· count/age |
| `Ctrl+U` on | 预览 · **完整 session id（不截断）** · count/age |

否决：常驻 UUID（挤占预览）；仅日志/调试旁路可见 id（排障摩擦大）。

## Keybinding

- 新 id：`app.session.toggleId`
- 默认和弦：`ctrl+u`
- 与既有 Resume 键并列：`ctrl+s/n/p/r/d`；不占用 `ctrl+p`（path）

## Layout math

```
fixed = prefix + [id_col + gaps if show_id] + meta
preview_budget = min(width - fixed, floor(width * 0.60))
```

显示 id 时优先压缩 preview；id 永不 `truncate_to_width`。

## Surface docs

- 产品视觉 SSOT：`src/app/tui/design/session-resume.md`
- 键位表：`keybindings.md`
- Playground：默认 / id-on 双态静图 + fixtures（apply 阶段）

## Specs

- `atm10`：行字段 + 默认隐藏 + 显示不截断 + 预览比例
- `ati29`：`Ctrl+U` toggle
