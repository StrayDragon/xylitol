# designing Web：骨架与双轨

权衡见 [`research/stack-survey.md`](./research/stack-survey.md)。本文只钉本切片落地形状。

## 两源

运行时真值 = 产品代码。意图 + 固定态 = `src/app/tui/designing/`。浏览器 cell-grid **不是** 产品真值（layoutlib ≠ 真机）。

## 目录

数据与模块靠近产品面；bun 应用 `package.json` 只在 `designing/app/`，不进 Cargo workspace、不进 `packages/xylitol-tui`。

```text
src/app/tui/designing/
  AGENTS.md
  app/          # Vite vanilla TS
  modules/
  generated/    # 勿手改
```

## 作者格式

YAML state：`must_contain` / `must_not_contain` + `lines: [{ text, token }]`。`token` 映射 DESIGN 色名（`muted` → `var(--muted)`）。可选 `cols:`。禁止 hex。

intent.md：可观察 MUST + 词表；禁止钉 Rust 路径/类型/行数。

## Token

SSOT 仍是 `DESIGN.md` frontmatter。`sync_tokens.py` 双写 playground + `designing/generated/`，直到拆除日只留后者。`just check-tui-tokens` 两边都对。

## 双轨拆除

activity-fold 闸改 YAML 后，HTML 模板不再对该模块做 `must_contain`。全部槽迁完且 playground 闸不再解析 HTML → 删 `index.html`，`open-design-playground` 一次性改成 `open-designing`。禁止永久 alias。

## 与 c2200

PreviewInject / HostSession 目录不在本设计内。
