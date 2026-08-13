# designing Web：顶层交互设计稿

权衡见 [`research/stack-survey.md`](./research/stack-survey.md)。

## 真值

运行时真值 = 产品代码。`designing/` 是给人看的交互设计稿（固定态 + 对齐 + 待办 + 备注），**不是**第二套视觉 SSOT。浏览器 cell-grid **不是**产品真值（layoutlib ≠ 真机）。

## 目录

升到仓库顶层，便于以后加 `designing/web/`（未交付则不建空目录）。`package.json` 只在 `designing/app/`。

```text
designing/
  AGENTS.md
  app/                 # bun + Vite vanilla TS
  generated/           # 勿手改
  tui/modules/<id>/
    draft.yaml
    intent.md
    states/*.yaml
```

## 作者格式

- `draft.yaml`：摘要、对齐（chrome / item / todo-bar）、todos、可选 `keys`（仅组件自有键）、备注。
- YAML state：`must_contain` / `must_not_contain` + `lines: [{ text, token, rev? }]`。禁止 hex。
- intent.md：可观察 MUST；禁止钉 Rust 路径/类型/行数。
- **无独立快捷键模块**。

## Token

SSOT 仍是 `DESIGN.md` frontmatter。`scripts/sync_tui_tokens.py` **只写** `designing/generated/`。`just check-tui-tokens` 对生成物 + `Palette`。

## 拆除（本切片已做）

旧 `src/app/tui/design/playground/` 删除；`open-design-playground` 一次性改为 `open-designing`（不留永久 alias）。`design/*.md` 缩成指针。闸只走 `scripts/check_tui_designing.py`。

## 与 c2200

PreviewInject / HostSession 目录不在本设计内。
