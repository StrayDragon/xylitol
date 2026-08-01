---
depends_on:
- c1800-add-tui-chrome-toast
branch: sdd/c1810-add-tui-chrome-footprint
base_sha: c645b0465bfc3df2e30225ef06b1940c5ee5e245
checkpointed: true
checkpoint_sha: c645b0465bfc3df2e30225ef06b1940c5ee5e245
---

# Chrome Footprint：终端高度预算地基

> busy 下打开列表槽时 `Working` spinner 易出屏；根因是 content-end 视口 + 高槽 emit，非 z-order。
> 本变更在 **app layout** 建立可扩展的高度 reserved 表，按 `term_rows` 预算各 `EditorSlot` 的 `max_visible`。
> **不**改 xylitol-tui content-end 视口语义；**不**默认引擎钉 chrome。

## Why

产品壳已有 queue / toast / status / 多列表槽，但 `max_visible=10` 等硬编码与真实终端高度脱节。短终端 + Resume/树时，贴底列表把 status 顶出视口上沿。未来再加 banner 类 band 只会更挤——需要 **单表 SSOT**，而不是逐槽改常量。

## 概念（调研结论 · 收进本提案）

`UiRoot` 序：`loaded-resources → scrollback → queue → toast → status → slot → footer`。

引擎：`viewport_top = max(0, content_len - term_rows)`（content-end，对齐 pi）。

```text
Working 在屏内 ⇔ term_rows ≥ slot_lines + footer(1) + status_busy_lead
# busy status = 前导空行 + Working → 按 2 行 reserved（见 status.md / atc12）
```

| 不推荐 | 原因 |
|---|---|
| 只改 Resume `MAX_VISIBLE` | 治一槽；magic number 继续增殖 |
| Footer 复述 Working | 无 spinner；不解决根因 |
| 引擎 Bottom-Chrome Pin | 与 pi 视口分叉；差分复杂；delayed 备选 |

## 已拍板

| 项 | 决定 |
|---|---|
| 地基 | **Chrome Footprint Manifest**（app `UiRoot`/host）+ term-aware 槽预算 |
| flex 消费者 | 仅 EditorSlot 内列表/树 **body** `max_visible` |
| 引擎 | 保持 content-end；本波 **不** pin |
| 正交 | 长 scrollback CPU 切片（c1505 等）另案 |

## What Changes

- host 注入 `term_rows`；resize 失效预算
- Manifest：lower chrome `min_rows`（queue/toast/status/footer…）+ slot 头行
- Resume / Tree / Models / MCP / Themes / Import：去掉硬编码 10，body 顶 = budget（≥1）
- `design/chrome-footprint.md`（或扩 layout 设计文）
- live specs：`app-tui-chrome` 新 req（atc23）
- harness：解 ignore 短终端 busy+Resume 视口仍含 `Working`

## Capabilities

- `app-tui-chrome`（atc23）

## 测试缝（apply 用）

| 缝 | 断言 |
|---|---|
| 短终端（16 行）busy + Resume 满列表 | content-end 视口窗内含 `Working` |
| 同条件 Models 开槽 | 同上 |
| 高终端（≥24） | 行为不回归；列表仍可滚动窗口化 |
| toast/queue 非空 | reserved 计入后 status 仍可见（或预算再缩 body） |

## Impact

- 窄终端列表更短，status/toast 可读
- 风险：过瘦列表；MUST `max_visible ≥ 1` 并保留滚动指示

## Out of scope

- 引擎三区 / pin chrome
- scrollback 行切片（CPU）
- 改 spinner 语义 / 下轮预告布局
