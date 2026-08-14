---
depends_on: []
skip_specs_landing: true
branch: c2230-tui-designing
base_sha: 8e560211a8cfcf8d5de476eea93f5e5a9843b1c2
checkpointed: false
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

## 目录落点（选定，不两套）

```text
src/app/tui/designing/
  AGENTS.md                 # 人/Agent 分界
  app/                      # bun + Vite vanilla TS（package.json 只在这里）
  modules/<id>/
    intent.md
    states/*.yaml
    preview.ts              # 组装该模块 cell/span
  generated/                # AGENT-INDEX.md、tokens.css/js；勿手改
```

不放 `tools/tui-designing/`（Agent 要靠近产品面）。`packages/xylitol-tui` **不**平行维护 design HTML。

## 旧 playground / 旧 md 拆除条件

短双轨 **允许**，禁止无限期兼容。

| 阶段 | 何时 | 做什么 |
|---|---|---|
| 本切片 | activity-fold 闸走 YAML states；其它槽仍解析 `index.html` | 保留 `playground/index.html`；禁止把槽复制成第三份 |
| 迁槽 | 每迁走一个槽 | 删 HTML 里对应 JS 模板 / section；`design/<comp>.md` 改成一行指针后删正文（Pre-0.0.1 无 SemVer 读者） |
| 切闸 | `scripts/check_tui_design_playground.py` **不再**解析任何 HTML 模板，且 `check_tui_designing.py` 覆盖全部已迁模块 | **删除** `playground/index.html`；`just open-design-playground` 改为 alias `open-designing`（一次性改调用点，不留永久转发） |
| tokens | 无槽再读 `playground/tokens.css` | `sync_tokens.py` 停止写 playground；只写 `designing/generated/` |

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

## 迁移清单（本切片只勾 activity-fold）

- [x] Activity fold（本切片）
- [ ] Palette / tokens（管道本切片接通；槽仍旧）
- [ ] Chrome toast · Pending · Status/footer 词表
- [ ] Transcript / expandable
- [ ] Models · Tree · Mcp · Resume · Compaction
- [ ] Tool · Diff · Markdown · Widgets · Atoms · Ask
- [ ] Full shell · Layout · Keybindings
- [ ] `session-tree-vs-pi.md` → research 或缩进 intent（对照文，不迁成模块）

顺序：tokens/palette → chrome 词表 → transcript/expandable → pickers → 整壳。

## Impact

- Agent 改折叠：读 `designing/modules/activity-fold/`，不吞 2.6k HTML。
- 人类：`just open-designing` / `bun run --cwd src/app/tui/designing/app dev`。
- qa：旧 playground 闸对其它槽仍绿；activity-fold 走 YAML；新 `check_tui_designing.py` 进 `just qa`。

## Further Notes

栈选定与否决表：[`research/stack-survey.md`](./research/stack-survey.md)。
