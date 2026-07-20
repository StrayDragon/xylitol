---
change_id: c1410-add-app-tui-tokenizer-consent
title: "TUI 词表下载知情确认（Tokenizer M2）"
status: purpose-draft
priority: 1410
depends_on:
  - c1380-add-cli-tokenizer-cache
author: agent
track: B
wave: tokenizer-consent
domain: app-tui
apply_band: P1-after-docs-m1
---

# c1410-add-app-tui-tokenizer-consent

> **purpose-draft（钉 M2 产品意图；升 full / attach 后再实现）**
> M1 CLI（`xylitol tokenizer …`）已归档并迁入 architecture。本 change 只补 **TUI 面**知情确认 / 跳过，与 CLI **共用同一缓存与配置映射**。

## Why

1. architecture / roadmap：CLI 已开箱；交互面仍缺「将下什么、多大、存哪」的确认，首次需要精准计数时易困惑。
2. 若只靠用户记 CLI，TUI 默认会话会长期停在启发式「约」，与 provenance 诚实叙事不一致。
3. 下载副作用与缓存语义已在 bridge + CLI；TUI 应是 **薄确认 + 委托**，禁止第二套 HTTP / 缓存实现。

## Purpose（升 full 时）

当 TUI 判定「当前模型需要 HF 词表且本地 missing」且产品策略要求知情时：

| 用户动作 | 结果 |
|---|---|
| **同意** | 走与 CLI 相同的 opt-in download（含 `HF_ENDPOINT`）；成功后本地计数可用 |
| **跳过** | 不下载；继续会话；计量降级并诚实标注 |
| **取消 / Esc** | 同跳过（不阻断聊天） |

MUST NOT：估计热路径静默下载；强塞无法跳过的阻塞且无说明。

## 开放问题（升 full 前钉）

1. **同意粒度**：本机一次 vs 每模型一次 vs 每会话一次？
2. **UI 落点**：host overlay / Choice 类确认 vs footer 行动点？（`src/app/tui` Choice stub 仍冻结——需明确是否解冻窄用或另开 overlay）
3. **触发时机**：选模型时？首次 estimate？显式 slash（如 `/tokenizer download`）？

## What Changes（升 full 时）

- `src/app/tui/`：确认 UX + pending → effects 调 Driver/CLI 等价 API（经 seam，不 reach infra HTTP）
- 可选：`protocol` / Driver 暴露「tokenizer status / download」只读端口，避免 TUI reach `infra`
- Specs：`app-tui-*`（或扩展既有）+ 与 ce15/paa6 对齐的场景
- Harness：同意 / 跳过两条合成路径
- **不**做 Web M3；**不**改 CLI 动词树（c1390 另轨）

## Capabilities（意向）

- `app-tui-host` / `app-tui-commands`（待钉）
- 复用既有 `cli-entry` ce15 语义、`package-ai-bridge-accounting` paa6/paa8

## Impact

- 用户：TUI 内可知情下载或跳过
- 兼容：CLI 行为不变；缓存路径不变
- 风险：解冻 Choice / 新 overlay 与 busy Esc 策略交互

## Ethics

- risk_level: medium（联网下载，须明示）
- prohibited: 静默下载；无跳过的强制墙
- required_evidence: harness 同意/跳过；估计路径仍无 download

## 下一步

1. 钉开放问题 1–3
2. 新 feature 分支 → 充实 design/tasks + live specs → `change attach` → apply
3. 闭环：脏树 `finalize` → **一次** commit → merge

## 非目标

- Web 管理（M3）
- `revision` / `HF_TOKEN` / nested `file` 列举修补（可另 quick/小 change）
- c1390 surface 动词
