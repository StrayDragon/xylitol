---
depends_on:
- c1780-update-tui-busy-instant-lists
branch: sdd/c1800-add-tui-chrome-toast
base_sha: fd765a6b2b1320a60ca3a413569deb2f4630c3dc
checkpointed: true
checkpoint_sha: fd765a6b2b1320a60ca3a413569deb2f4630c3dc
---

# 壳层通告（chrome toast）：固定位瞬时提示

> 兑现词汇表预留的 **壳层通告**；busy Resume 拒 switch 等不再用主滚动区 `ScrollNotice`。
> 落点已拍：**status / spinner 上方一行**；定时自动消失。
> Roadmap 背景：键位与命令发现 M0 落地后的体验修正。

## Why

`ScrollNotice` 进 `UiEntry` 主滚动区：流式长文时提示被顶出视口；尾随又压 LLM 正文。架构已写明「能进 chrome 的别做成滚动提示」，并预留 **壳层通告 / toast**——本变更把它做实。

## 概念对齐（给读者）

| 层 | 是什么 | 代码直觉 |
|---|---|---|
| **主滚动区 / transcript** | 可滚的对话时间线 | `UiModel.entries: Vec<UiEntry>` |
| **`UiEntry`** | **主滚动区一行条目**（user/assistant/thinking/tool/…/`ScrollNotice`） | `bridge/model.rs`；**不是** footer/status/槽 |
| **滚动提示** | 一种 `UiEntry`，仍占滚动时间线 | `UiEntry::ScrollNotice` + `push_scroll_notice` |
| **chrome** | 壳：status、footer、下轮预告、槽、loaded-resources | layout 渲染，**不进** `entries` |
| **壳层通告（本波）** | 非 scrollback 的短暂固定提示 | **新建** toast 槽（非新 `UiEntry` 变体） |

**结论**：不要加 `UiEntry::FixedNotice`——那仍是正文区条目；固定位瞬时提示 = chrome toast。

## 已拍板

| 项 | 决定 |
|---|---|
| 落点 | status / spinner **上方**恰好一行（busy 有 status 时贴在其上；idle 时仍可在 editor 呼吸间距上方显示） |
| 寿命 | 显示后 **自动消失**（默认约 3–5s，可常量；新 toast 替换旧） |
| 首个调用方 | c1780 busy Resume **switch / rename / delete** 拒闸文案 A → **改走 toast**，MUST NOT 再 `push_scroll_notice` |
| 非目标 | 角区浮动、把全部 C 类诊断迁出 ScrollNotice、改 spinner 语义 |

## What Changes

- layout：chrome toast 行（`{colors.warning}` + `Error: ` 前缀）；host API `push_chrome_toast`
- 改写 c1780 拒闸路径：toast 替代 ScrollNotice
- live specs：`app-tui-chrome` atc22；`atm10` / `ati29` 改落点
- 词汇表 / `design/chrome-toast.md`：激活壳层通告现行说明
- harness：拒闸无新 ScrollNotice；toast 可见且可超时清除

## Capabilities

- `app-tui-chrome`（新 atc2x）
- `app-tui-commands`（atm10）
- `app-tui-input`（ati29）

## 测试缝（已对齐 · apply 用）

复用产品 TUI harness / `HostSession`（与 c1780 同路）：

| 缝 | 断言 |
|---|---|
| busy Resume Enter switch | MUST NOT 新增文案 A 的 `ScrollNotice`；toast 槽含文案 A |
| busy rename/delete | 同上 |
| bang-busy switch | 同上 |
| toast TTL | 推进时钟 / idle_tick 后 toast 清空（或等价可测钩） |
| 既有 travel/fork 尾随 ScrollNotice | 不回归 |

## Impact

- busy 拒操作时提示固定可见，不干扰流式正文
- 风险：toast 与 status 两行增高 chrome；须控制为 1 行 toast + 既有 1 行 status

## Out of scope

- `UiEntry::FixedNotice`
- 右上角角区实现（可作后续支线）
- 把 `/reload` refused 等全部迁 toast（本波仅 c1780 拒闸；MAY 列迁移清单）
