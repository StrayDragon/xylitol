---
change_id: c1400-prefer-config-yaml-secret-env
title: 配置仅 config.yaml + secret.env；移除 config.local.yaml
status: full
priority: 1400
depends_on: []
author: agent
track: B
wave: config-ergonomics
domain: runtime-config
apply_band: C-remove-local
branch: feat/c1400-prefer-config-yaml-secret-env
base_sha: cef465c1c4b832e8168dde66057dcbd69be2a01e
checkpointed: true
checkpoint_sha: cef465c1c4b832e8168dde66057dcbd69be2a01e
---

# c1400-prefer-config-yaml-secret-env

> **破坏性（未发布，一步到位）**：只支持 `config.yaml` + `secret.env`；**不再加载** `config.local.yaml` / `.yml`；**不提供**内容迁移工具。

## Why

`.local` 与 `config.yaml` + `secret.env` 职责重叠，协作易把共享项塞进 gitignore 文件。Pre-1.0 未发布 → 直接删层，不留逃生舱、不写迁移脚本。

## 实现档（已钉）

| 档 | 本 change |
|---|---|
| A 仅文档降级 local | ❌ |
| B 仍合并 + doctor hint | ❌ |
| **C 不再合并 local** | ✅ |

## Purpose

1. 加载链 MUST 仅为：
   ```text
   ~/.config/xylitol/config.yaml
   → 项目 .xylitol/config.yaml
   → --config
   + secret.env（global ← project；OS env 不被覆盖）
   ```
2. MUST NOT 读取或深合并 `config.local.yaml` / `config.local.yml`（global 与 project）。
3. 磁盘上若仍存在上述文件：MUST 忽略其内容；MAY 在 `resources doctor`（或加载诊断）给一行「已忽略」提示——**MUST NOT** 自动把内容写入 `config.yaml` / `secret.env`。
4. 文档 / example / 缺模型文案：只讲 `config.yaml` + `secret.env`；MUST NOT 再描述 local 覆盖层。
5. 遗留 migrate（`~/.xylitol` → XDG）：MUST NOT 再复制 `config.local.*` 为目标 AppConfig 层（或复制后亦被 loader 忽略——以实现为准，合约以「不生效」为准）。
6. 家目录：AppConfig SSOT = `~/.config/xylitol/`；`~/.xylitol/` = 数据目录。

## What Changes

- `src/infra/config/loader.rs`：去掉 local 层
- `migrate.rs` / paths 注释：对齐
- 文档、`configs/example.yaml`、`.xylitol/secret.env.example`、TUI 缺模型文案、本仓 `.xylitol/config.yaml` 头注释
- 测试：e2e/pty 等改用 `config.yaml`；loader 单测证明 local 不生效
- live `runtime-config`：`rc20`/`rc21`

## Out of scope

- 自动迁移 local → config.yaml/secret.env
- 保留 `XYLITOL_ALLOW_CONFIG_LOCAL` 开关
- schema 大扫除；迁 tokenizers 缓存根

## Capabilities

- `runtime-config`（modify）

## Ethics

- risk_level: medium（破坏依赖 local 的本机布局）
- prohibited_actions: 静默把 local 内容合并进可提交文件；保留「默认仍读 local」的后门
- required_evidence: loader 测 + 文档无 local 主叙事；doctor/忽略行为有测（若实现提示）
- escalation_policy: 无——用户已确认破坏性一步到位
