---
change_id: c1380-add-cli-tokenizer-cache
title: "CLI tokenizer 词表缓存：status / download / clean（知情同意 + HF 镜像）"
status: full
priority: 1380
depends_on: []
author: agent
track: B
wave: tokenizer-consent
domain: cli
branch: feat/c1380-add-cli-tokenizer-cache
base_sha: a481fdb0bb9ffe675daa024edab5f5d7c9310105
checkpointed: false
---

# c1380-add-cli-tokenizer-cache

## Why

1. 本地 HuggingFace `tokenizer.json` 已有 bridge 侧 **opt-in 下载 API**（`HfTokenizerCache::download_opt_in`）与合约 **paa6**（默认禁止静默拉大文件），但**产品 CLI 无快捷路径**：用户无法知情同意、查看缓存、清理磁盘。
2. architecture 已兑现用量 provenance；roadmap「Tokenizer 精准计量」M1 只差 **CLI 同意流 / 管理面**。
3. 开源/本地友好模型通常**没有** Builtin tiktoken；必须能声明「用哪份词表」，并在国内等环境经 **`HF_ENDPOINT`（如 hf-mirror）** 改默认 HF 基址，否则 download 不可用或极慢。
4. 历史意向（c1030 下游 `c1050-add-cli-ai-bridge-tokenize`）未落地；本 change 用 **c1380** 承接。

## Purpose

交付顶层 **ops** 动词 **`xylitol tokenizer`**（跨面管理；**不**挂在未来的 `tui` 子树下——统一入口树见后续 **c1390**），叶子：

| 叶子 | 用户意图 |
|---|---|
| `status` | 缓存根、条目、某模型 `builtin` / `cached` / `missing` / `unmapped` / `local` |
| `download` | 显式同意后拉取；打印将下什么 / URL 基址 / 落盘路径；失败仍可用（降级计量） |
| `clean` | `--all` 或按 model/target 清理 |

估计路径仍遵守 paa6：无缓存且未 download → **不**静默联网。

## 配置：用户要不要配 model / tokenizer path？

**要，对非 Builtin 模型。** 解析优先级（设计钉死）：

```text
ModelEntry.tokenizer（用户配置）
  → registry builtin 启发式（仅常见 OpenAI 族）
  → unmapped（须 CLI 显式 owner/repo，或先补配置）
```

| 模型情况 | 用户是否必须配置 | 行为 |
|---|---|---|
| 常见 OpenAI 族（builtin tiktoken） | **否** | `status`=`builtin`；`download` 提示无需下载 |
| 开源/兼容端需 HF 词表 | **是**（或 download 时显式 `owner/repo`） | 配置 `tokenizer.huggingface` 或 CLI target |
| 已有本地 `tokenizer.json` | **是**（`tokenizer.local.path`） | 不下载；直接离线加载 |
| 未映射且无 CLI target | — | `download`/`status --model` **明确错误**，不猜厂商、不盲下 |

建议 YAML 形状（本波落地最小字段；schema/example 见 tasks）：

```yaml
models:
  models:
    qwen-local:
      provider: openai-compatible   # 示例
      model: Qwen/Qwen2.5-7B-Instruct
      tokenizer:
        # 三选一（互斥）
        huggingface:
          repo: Qwen/Qwen2.5-7B-Instruct   # HF model id / repo
          file: tokenizer.json            # 可选，默认 tokenizer.json
        # local:
        #   path: /path/to/tokenizer.json
        # builtin: true                   # 显式强制 builtin（罕用）
```

**不是**「model page path」漫游浏览器页，而是 **HF repo id + 文件名**（或本地文件路径）。「model card URL」解析 → **后置 / 待完善**。

## HF 镜像与基址（本波 MUST）

| 来源 | 规则 |
|---|---|
| 默认 | `https://huggingface.co` |
| 环境变量 **`HF_ENDPOINT`** | 若设（如 `https://hf-mirror.com`），download URL 基址改用它（去尾 `/`） |
| URL 形状 | `{base}/{repo}/resolve/main/{file}`（revision 固定 `main`；可配置 revision → **待完善**） |
| 展示 | `download` 确认摘要 MUST 打印实际 base（用户可知是否走镜像） |

兼容说明：社区亦见 `HF_HUB_ENDPOINT`；本波以用户指定的 **`HF_ENDPOINT` 为权威**；若两者皆设，**HF_ENDPOINT 优先**；仅有 `HF_HUB_ENDPOINT` 时 MAY 回退读取（实现时单测钉死，避免静默不一致）。

私有仓 / `HF_TOKEN` 鉴权 → **待完善**（本波可不实现；失败时错误可读）。

## What Changes

- CLI：`CliCommand::Tokenizer` + `src/app/cli/tokenizer.rs`；早退；**不** bootstrap 模型会话
- Bridge：`list` / `remove` / `cache_root`；原子 download；**按 `HF_ENDPOINT` 拼 URL**
- Registry + 配置：`resolve_tokenizer` 读 ModelEntry.tokenizer 覆盖；builtin 启发式保留
- Specs：`cli-entry` ce15；`package-ai-bridge-accounting` paa8（+ HF 基址 / 配置映射相关句）
- BDD / 单测：status、download 同意落盘、clean、无静默下载、镜像基址拼装

## Capabilities

- `cli-entry`（modify）
- `package-ai-bridge-accounting`（modify）
- `runtime-config`（modify：ModelEntry 可选 `tokenizer` 字段——最小 schema）

## Out of scope / 待完善（明确不阻塞 M1 主路径）

| 项 | 说明 |
|---|---|
| TUI / Web 确认与管理 | roadmap M2/M3 |
| GGUF / 非 `tokenizer.json` | 原 c1045 意向 |
| 从 model card / 网页自动发现词表 | 需稳定启发式；另 change |
| revision / branch / commit 钉死 | 本波固定 `main` |
| `HF_TOKEN` 私有仓 | 另 change |
| 配置预声明「已同意下载」跳过 TTY 确认 | 另 change |
| CLI 表面命名空间 `tui` / `print` | **c1390**（本波仅保证 `tokenizer` 为顶层 ops） |
| 写入 architecture 未兑现正文 | 归档后再迁 |

## Ethics

- risk_level: medium
- prohibited_actions: 估计热路径静默联网；未告知 URL/落盘路径的强制下载；伪造 LocalTokenizer provenance；无映射时猜测下载目标
- required_evidence: CLI/单测证明仅 download 落盘；`HF_ENDPOINT` 改变请求基址；配置/显式 repo 才能解析 HF 源
- escalation_policy: ModelEntry.tokenizer schema 若与现有配置加载冲突，升级用户确认字段名

## Depends

- []（依赖已归档 accounting / paa6；非活跃 change id）
- 后续：**c1390-add-cli-surface-verbs**（统一入口树；不阻塞本 change apply）
