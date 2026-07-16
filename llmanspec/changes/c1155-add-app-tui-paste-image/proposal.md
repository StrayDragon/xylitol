---
change_id: c1155-add-app-tui-paste-image
title: "产品 TUI：粘贴图片进对话（含临时路径 fallback）"
status: purpose-draft
priority: 1155
depends_on: []
author: agent
track: R
wave: paste
domain: app-tui
---

# c1155-add-app-tui-paste-image

## Why

pi 支持 Ctrl+V 粘贴图片。xylitol domain 有 Image part；clipboard 有读图；包层 Image 完整路径裁剪（D11）。产品需可提交图片附件，并在终端不支持 / 读失败时 **fallback（如写入临时路径再 `@` 引用或 read）**。

## Purpose

产品 editor 粘贴图像 → 纳入待发消息（base64 或临时文件路径策略升格钉死）；provider 映射经既有 Image 部分；失败提示 + fallback；不回退 D11「包内完整 Kitty encode」决议，除非另开包 change。

## What Changes（升格 full 时）

- host InputListener / paste 分支
- 临时文件生命周期与清理
- Fake/harness 可测路径
- delta：`app-tui-input` · `infra-clipboard` · `domain-message`

## Capabilities

- `app-tui-input`（modify）
- `infra-clipboard`（modify）
- `infra-image`（modify）

## Out of scope

- 完整 Kitty/iTerm Image 组件（D11）
- 拖放多文件（可后置）

## Ethics

- risk_level: medium
- prohibited_actions: 临时文件落在可预测世界可读路径且不清理；把大图无上限塞进 session
- required_evidence: 粘贴成功/失败/fallback 三路径；大小上限
- escalation_policy: 仅路径引用 vs 内联 base64 需确认（影响 provider）

## Depends

- []
