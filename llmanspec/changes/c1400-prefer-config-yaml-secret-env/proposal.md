---
change_id: c1400-prefer-config-yaml-secret-env
title: "配置偏好：config.yaml + secret.env；收敛 config.local.yaml"
status: purpose-draft
priority: 1400
depends_on: []
author: agent
track: B
wave: config-ergonomics
domain: runtime-config
apply_band: P2-after-c1380
---

# c1400-prefer-config-yaml-secret-env

> **purpose-draft（钉产品偏好，后升 full / apply）**
> Pre-1.0 配置可以乱，但心智要清晰：**可分享的进 `config.yaml`，密钥进 `secret.env`，少用/弃用 `config.local.yaml`。**

## Why

今日加载链同时支持：

```text
~/.config/xylitol/config.yaml[+.local] → 项目 .xylitol/config.yaml[+.local] → --config
+ secret.env（global ← project）
```

问题：

1. **`config.local.yaml` 与 `config.yaml` + `secret.env` 职责重叠**——本机覆盖既可塞 local YAML，也可把密钥放 secret / 非敏感共享放 config.yaml，用户不知偏哪边。
2. 仓库协作时 **`.local` 被 gitignore**，共享模型/tokenizer/base_url 容易误放 local，换机器又丢。
3. 家目录 `~/.config/xylitol` 与遗留 `~/.xylitol` 双轨 + migrate，排障成本高（数据目录仍可留在 `~/.xylitol`：sessions / tokenizers 缓存）。

产品更偏爱：

| 文件 | 放什么 | git |
|---|---|---|
| **`config.yaml`** | 模型、tokenizer 引用、agents、非密钥行为 | 项目侧 **可提交** |
| **`secret.env`** | API key 等密钥；YAML 用 `{{ secret.KEY }}` | **不提交**（已有 example） |
| **`config.local.yaml`** | 可选逃生舱；**不推荐**作默认路径 | 忽略 |

## Purpose（升 full 时）

1. **文档 / AGENTS / example**：主叙事改为 `config.yaml` + `secret.env`；`.local` 标为「可选本机覆盖，能不用则不用」。
2. **行为（候选，升 full 时钉一条）**：
   - **A（软）**：加载顺序不变；仅文档与警告（启动时若只用了 local 而无 base config.yaml 则 hint）。
   - **B（中）**：项目侧若存在 `config.yaml`，仍合并 local，但 **help/doctor 提示 local 已过时**。
   - **C（硬，可后置）**：默认忽略 `config.local.yaml`，需 `XYLITOL_ALLOW_CONFIG_LOCAL=1` 才合并。
3. **家目录心智**：AppConfig 全局 SSOT = `~/.config/xylitol/`；`~/.xylitol/` = 数据（sessions / tokenizers / logs），**不再**鼓励在 `~/.xylitol/config.yaml` 放 AppConfig（migrate 保留兼容）。
4. **runtime-config** 合约补一句偏好（MUST 叙事级或 SHOULD）。

推荐升 full 时先做 **A→B**；C 另切片。

## What Changes（升 full 时）

- docs：`docs/architecture/配置与档案.md`、`configs/example.yaml`、`.xylitol/secret.env.example` 文案
- 可选：`resources doctor` / 启动 warning
- live `runtime-config` requirement + feature
- **不**在本 draft 删除 loader 对 local 的支持（除非选 C）

## Out of scope

- 重做整棵配置 schema / 1.0 配置大扫除
- 把 tokenizers 缓存挪出 `~/.xylitol/`
- c1380 tokenizer 行为本身

## Ethics

- risk_level: low
- prohibited_actions: 把密钥写进可提交 `config.yaml`；静默删除用户 `secret.env`
- required_evidence: 文档与 example 一致；若改加载行为则有 BDD/单测
- escalation_policy: 若选 C（默认忽略 local）须用户确认

## Depends

- []（可与 c1380 并行文档；实现可等 c1380 归档）
