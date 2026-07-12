# design — c475 app-tui-chrome

## 决策

| 项 | 选择 | 理由 |
|---|---|---|
| 主题 | 固定 `Palette::dark()` | MVP；auto/`/theme` 留给 demo / 后置 |
| Glyph | `XYLITOL_TUI_GLYPH_SET=ascii\|unicode`（默认 unicode） | 对齐 DESIGN；无字体探测 |
| Status | 独立 `Text` 槽；idle 空串 → 0 行 | 复用包 `Text` 空文本行为；Working 不进 footer |
| Footer | `cwd · model` + 可选队列前缀；右侧截断 | 对齐 `design/footer.md`；树槽自带操作提示，footer 不换键墙 |
| cwd/model | host 构造时注入 | Driver 有 model；cwd 取 `current_dir`（HOME→`~`） |

## 布局（Ready）

```
transcript
status          # 0 或 1 行
──── border ────
editor | tree
──── border ────
footer          # 1 行 dim
```

## 触点（最小）

- 新：`theme.rs`、`glyphs.rs`
- 改：`ui_root.rs`、`mod.rs`、`host.rs` / `run` 注入 chrome meta、`bridge` scrollback 前缀（可选经 UiRoot）、`tests.rs`
- 不改：c491 stub 行为；bridge 事件语义

## 非目标

- Markdown/Diff 真渲染（仍占位行）
- 产品 theme 切换 UI
