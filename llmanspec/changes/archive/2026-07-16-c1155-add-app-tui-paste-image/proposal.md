---
change_id: c1155-add-app-tui-paste-image
title: "产品 TUI：粘贴图片落盘插路径 + read 回真图（对齐 pi）"
status: full
priority: 1155
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: paste
domain: app-tui
ethics:
  risk_level: medium
  prohibited_actions:
    - 在 Editor 内嵌大段 base64
    - 提交时把粘贴路径再复制成 user Image part（偏离 pi）
    - app/tui 直达 infra::clipboard / infra::image（绕过 Driver）
  required_evidence:
    - harness：剪贴板有图 → editor 出现 tempfile 绝对路径；提交仍为纯文本
    - read 读图片路径 → tool result 含 AgentPart::Image（resize 后 base64）
    - 剪贴板无图 / 读失败 → 短错误或不崩
  escalation_policy: provider 所需 base64 只在 read 工具产出；粘贴仅落盘+路径文本
---

# c1155-add-app-tui-paste-image

## Why

pi 交互粘贴：**写 tempfile → 编辑器插入绝对路径纯文本 → 提交仍是 text**；模型要看图时调 `read`，此时才 `processImage` → `ImageContent.data`。xylitol 已有 clipboard/image infra，但产品 TUI 未接线；且 `read` 对图片只回占位符（注释误称 like pi）。

## What Changes

1. **粘贴（对齐 pi）**：`app.paste.image`（默认 Ctrl+V）经 Driver 读剪贴板图 → tempfile（uuid）→ Editor 插入**裸绝对路径**（无 `@`、无 base64）。无图/失败：短 Error，不崩；MUST NOT 从 app/tui reach infra。
2. **提交**：仍走既有纯文本 `run` / `steer` / `follow_up`。**MUST NOT** 在提交时 path→user `Image` part。
3. **read 回真图**：图片路径经 `resize_image` 产出 `AgentPart::Image`（+ 短 text note）写入 tool result；扩展工具输出可携 parts（默认仍包一层 text）。
4. **Harness**：ScriptedDriver 可注入假剪贴板图；断言 editor 路径与 read 多部件。

## 已钉策略

| 决策 | 选择 |
|---|---|
| Editor | 绝对路径纯文本（对齐 pi；**不用** `@` 前缀） |
| 提交 | 纯文本；**不**转 Image |
| base64 | 仅 `read`（及 CLI 等既有入口） |
| Kitty Image | **不做**（D11） |
| temp 清理 | 对齐 pi：**不**强制提交后删除 |
| arch | 仅经 `Driver` 触达 clipboard |

## Capabilities

- `app-tui-input`（add ati37）
- `infra-clipboard`（add c7）
- `infra-image`（add i5）
- `agent-tools`（add t21 read-image-parts）

## Out of scope

- 提交时 user multipart Image
- Kitty/iTerm 内联图组件
- 拖放多文件
- 强制清理 paste tempfile

## Impact

- `src/app/tui/host/*`、`effects`、`Driver`、`keybindings`
- `infra/clipboard` tempfile；`infra/image` path→ImageContent
- `XyTool` parts 输出；`infra/tools/read.rs`；react 写入 tool_result parts
