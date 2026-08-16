---
depends_on: []
skip_specs_landing: true
branch: c2230-tui-designing
base_sha: 8e560211a8cfcf8d5de476eea93f5e5a9843b1c2
checkpointed: true
checkpoint_sha: c8b1a1acf0e59a39e7fa35b5071aac658193d7f1
---

# TUI designing Web：意图静图收成两源

> **一句话**：把 `design/*.md` + `playground/index.html` 收成模块化 bun 应用 `designing/`；人类浏览器预览固定态，Agent 读短结构化上下文；产品运行时仍以代码为准。
>
> 调研：[`research/stack-survey.md`](./research/stack-survey.md)。

## Why

今天三套「看起来像设计」的东西互相脱轨：

| 源 | 给谁 | 问题 |
|---|---|---|
| `DESIGN.md` frontmatter | 人 + 闸 + `Palette` | 真正的数据；保留 |
| `design/*.md`（~29） | 人 + Agent | 编号 MUST 墙；钉实现细节；当 SSOT 读会过时 |
| `playground/index.html`（~2664 行） | 人（浏览器） | Agent 默认忽略；手写第二套 UI；不可模块化 |

再加上产品代码 = 第四源。本票 **不**把产品 paint 搬进浏览器；只把 **md + 巨石 HTML** 收成一个可预览、可模块化、对 Agent 便宜的 `designing`。

c2200 PreviewInject / HostSession-in-browser 是 **真 host 夹具**，与本票正交：那边管真机；这边管意图静图（layoutlib，不是真机）。禁止混 PR。

## 两源模型

| 真值 | 管什么 | 不是什么 |
|---|---|---|
| **(1) 产品代码** `src/app/tui/` + `xylitol-tui` | 运行时：格子比、差分绘制、鼠标、流式帧 | 设计意图散文 |
| **(2) `designing/`** | 意图（短 MUST）+ 结构化固定态 + tokens | 产品真值；不是 wasm TUI |

`DESIGN.md` 正文可缩成 designing Overview 或生成物（本切片保留 frontmatter 作 **唯一色板数据**）。浏览器预览 **不是** 产品真值。

## What Changes

本切片（可停）：

1. 调研 + 本提案（含拆除条件、atc4 草稿）。
2. bun 应用骨架：左模块列表、右固定态预览、token 驱动颜色。
3. Token 管道：一份 frontmatter → playground（双轨）+ `designing/generated/` + 既有 Palette 闸。
4. 首模块 `activity-fold`：压缩 intent + YAML states + cell-grid 预览（collapsed / envelope / expanded）。
5. Agent 阅读：`designing/AGENTS.md` supersede 过时「唯一视觉 SSOT / Agent 忽略 HTML」；`generated/AGENT-INDEX.md` + qa 过期检查。

**不做**：wasm `xylitol-tui`、嵌 `HostSession`、PreviewInject、`/debug` 侧栏、tui-pantry、`agent_demo`、Scene 框架、Figma、一次删光旧 md/HTML。

## 目录落点（现行）

仓库顶层（便于以后加 `designing/web/`；未交付不建空目录）：

```text
designing/
  AGENTS.md
  app/                      # bun + Vite vanilla TS（package.json 只在这里）
  tui/modules/<id>/
    draft.yaml              # 摘要 / 对齐 / todos / 组件键 / 备注
    intent.md
    states/*.yaml
  generated/                # AGENT-INDEX.md、tokens.css/js；勿手改
```

`packages/xylitol-tui` **不**平行维护 design HTML。无独立快捷键模块。

## 旧 playground / 旧 md 拆除（已做）

`src/app/tui/design/playground/` 已删；`open-design-playground` 一次性改为 `open-designing`（不留永久 alias）。`design/*.md` 缩成指针。闸只走 `check_tui_designing.py`。token 脚本只写 `designing/generated/`。

## atc4 产品级改写草稿（不落 specs）

现行（钉路径）：视觉 MUST 以 `DESIGN.md` 为索引、以 `design/*.md` 为组件级 SSOT。

**改写句（交给 c2210 或另开）**：产品 TUI 的视觉意图与固定态对照 MUST 来自 designing 模块（短可观察 MUST + 结构化状态）与产品 host 实现；浏览器静图 **不是** 运行时真值。MUST NOT 把未迁完的巨石 HTML 或过时组件散文当作实现真值。

`app-tui-chrome.feature` 里「组件索引列出 design/ 下核心文档路径」应改为「设计模块索引可列出每个表面的意图与状态」，不钉文件名。

## 与 c2200 正交

| | c2200 | 本票 c2230 |
|---|---|---|
| 真值 | 产品 `HostSession` + 夹具 | designing 意图 + 固定态 |
| 预览 | 真 host（PreviewInject） | 浏览器 cell-grid（layoutlib） |
| 禁止互吞 | 不把 HTML 当真机 | 不把 wasm TUI 当设计源 |

## Capabilities

无 live 合约落地（`skip_specs_landing: true`）。atc4 只出草稿。意图闸是脚本级，不是产品运行时行为。

## 迁移清单

- [x] Activity fold
- [x] Palette / tokens
- [x] Chrome toast · Pending · Status/footer 词表
- [x] Transcript / expandable
- [x] Models · Tree · Mcp · Resume · Compaction
- [x] Tool · Diff · Markdown · Widgets · Atoms · Ask
- [x] Full shell · Layout
- [x] 取消独立 Keybindings 模块（组件键留在各 draft.keys）
- [x] `session-tree-vs-pi.md` → 指针到 session-tree

## Impact

- Agent 改某表面：先读产品代码，再读 `designing/tui/modules/<id>/`。
- 人类：`just open-designing` / `bun run --cwd designing/app dev`（pathname `/tui/<id>/<state>`；右栏复制 handoff；←→ 切态、`?play=1` 对照 spinner）。
- qa：`scripts/check_tui_designing.py` 进 `just qa`；旧 playground 闸已删。

## Further Notes

栈选定与否决表：[`research/stack-survey.md`](./research/stack-survey.md)。
