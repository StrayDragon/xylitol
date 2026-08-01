---
version: "alpha"
name: "chrome-footprint"
description: "Term-aware reserved rows + EditorSlot list/tree body max_visible (atc23)."
tokens_from: "../DESIGN.md"
components:
  chrome-footprint:
    statusBusyRows: "2"
    statusIdleRows: "1"
    footerRows: "1"
---

# Chrome Footprint（高度预算）

> Token：本文件不另立色板；引用 [`../DESIGN.md`](../DESIGN.md)。
> Status：[`status.md`](./status.md)。Toast：[`chrome-toast.md`](./chrome-toast.md)。Queue：[`queue-steer.md`](./queue-steer.md)。
> 合约：`app-tui-chrome` **atc23**。实现：`layout/chrome_footprint.rs` + `UiRoot::apply_chrome_footprint`。

## 问题

引擎 content-end 视口贴底（`viewport_top = max(0, len − term_rows)`）。高列表槽把 **busy status**（含 `Working`）顶出视口上沿——**不是** z-order 遮罩。

## MUST

1. **SSOT**：产品 TUI MUST 用单表/单函数计算 lower chrome reserved 与各 flex 槽 body 预算；新 band = **加表项**，禁止各槽再写死与 `term_rows` 脱节的运行时顶（如硬编码 `10`）。构造期 soft default（如 `10`）允许，首帧/resize 后 MUST 被预算覆盖。
2. **Reserved（下缘）**：`queue`（非空时按实际行）+ chrome toast（0|1）+ status（busy=2 / idle=1）+ footer（1）。Upper（loaded-resources / scrollback）**不**计入 reserved。
3. **Body 顶**：`max_visible = max(1, term_rows − reserved − slot_header − slot_trailer)`。至少覆盖 Resume、Tree、Models、MCP、Themes、Import。
4. **Host**：MUST 在构造 / resize / 同步 paint 前注入 `term_rows`；render 前 MUST 应用预算。
5. **引擎边界**：MUST NOT 为本需求改 xylitol-tui content-end 语义或引入引擎级 bottom-chrome pin（另开变更除外）。

## 槽头参考（实现表）

| 槽 | header（约） | trailer（约） |
|---|---|---|
| Models / Themes / Import | 1 | 0 |
| MCP | 1 | 1 + diag 行 |
| Resume | 4（+status 行） | 0 |
| Tree | 4 | 1（scroll info） |

## 非目标

- Footer 复述 Working 作为主方案
- 三区引擎 / pin bottom chrome
