# Design: c1380 CLI tokenizer（全量）

## 在统一 CLI 入口中的位置

已与产品约定（**c1390** 落地表面树；本 change 先占 ops 位）：

```text
表面 surface     xylitol | xylitol tui … | xylitol print …
管理 ops         xylitol resources … | xylitol server … | xylitol tokenizer …
共享 flags       --model / --session / --config / --trust（各面可继承）
```

**`tokenizer` MUST 为顶层 ops**，MUST NOT 放在 `tui` 下（词表缓存跨 print/TUI/未来 Web）。

## 命令树（M1）

```text
xylitol tokenizer status [--model <id>]
xylitol tokenizer download <target> [--yes] [--file <name>]
xylitol tokenizer clean (--all | --model <id> | <target>)
```

| 参数 | 含义 |
|---|---|
| `--model <id>` | 配置别名 / 模型 id → `resolve_tokenizer` |
| `<target>` | 模型 id **或** `owner/repo`（含 `/` 视为 HF repo） |
| `--file` | 覆盖默认 `tokenizer.json`（与配置 `tokenizer.huggingface.file` 同义） |
| `--yes` | 跳过 TTY 二次确认 |

### 叶子语义

1. **`status`**：缓存根（默认 `~/.xylitol/tokenizers/`）、条目列表；`--model` 时一行：`builtin` | `local` | `cached` | `missing` | `unmapped`
2. **`download`**：调用=同意；TTY 打印摘要（**repo/file、HF base、落盘路径**）后确认；`--yes` 跳过；已缓存幂等；失败不留坏文件
3. **`clean`**：缺选择器 → usage 错误；目标不存在 → **0 + 提示**（幂等）；`--all` 清空缓存根下条目

### 不做

| 不做 | 原因 |
|---|---|
| `tokenizer consent` | 与 download 重复 |
| 并列 `list` | 并入 status |
| 顶层 `cache` 大伞 | 过早抽象 |
| flat `--prefetch-tokenizer` | 破坏动词树 |

## 配置模型 ↔ 词表（核心）

### 为什么要配

Builtin tiktoken **只覆盖**少量 OpenAI 族启发式。Qwen/Llama/DeepSeek 等要本地精确计数，必须知道 **HF repo（或本地 path）**——这不是「模型推理 base_url」，而是 **计量用词表源**。

### `ModelEntry.tokenizer`（最小）

```yaml
tokenizer:
  huggingface:
    repo: "org/name"
    file: "tokenizer.json"   # optional
# 或
tokenizer:
  local:
    path: "/abs/or/relative/tokenizer.json"
# 或
tokenizer:
  builtin: true
```

互斥：同时出现多种 → 配置加载失败（可读错误）。

### 解析顺序

```text
resolve_tokenizer(model_id):
  1. 若配置有 tokenizer.local     → LocalPath { path }（加载，不下载）
  2. 若配置有 tokenizer.huggingface → HuggingFace { repo, file }
  3. 若配置有 tokenizer.builtin     → Builtin（显式）
  4. 否则 builtin_tokenizer_for(model_id) 启发式
  5. 否则 None → unmapped
```

`download`：

- Builtin / Local → 不下载（0 + 说明）
- HuggingFace → 经 HF base 拉取到缓存键 `repo/__file`
- unmapped + CLI `owner/repo` → 允许一次性下载（仍 opt-in）；**建议**用户事后写入配置以免每次敲 repo
- unmapped 且无 repo → 错误，提示配置字段或 CLI 形状

### 与「model page path」的关系

用户说的「model page / tokenizer path」在本产品落为：

| 用户说法 | 本设计字段 |
|---|---|
| HF 模型页对应的仓库 | `tokenizer.huggingface.repo` |
| 词表文件 | `tokenizer.huggingface.file`（默认 `tokenizer.json`） |
| 本机已有词表 | `tokenizer.local.path` |

不爬 HF 网页；不把 chat `base_url` 当成词表 CDN。

## HF 基址与镜像

```text
hf_base =
  trim_trailing_slash(env HF_ENDPOINT)
  ?? trim_trailing_slash(env HF_HUB_ENDPOINT)   # 可选回退
  ?? "https://huggingface.co"

url = "{hf_base}/{repo}/resolve/main/{file}"
```

- `HF_ENDPOINT=https://hf-mirror.com` → 走镜像（本波 **MUST** 支持）
- 确认摘要打印 `hf_base`，避免用户不知仍打官方域
- 单测：注入 env 断言 URL 前缀；默认无 env 时为官方域

### 待完善（HF）

- [ ] revision / commit（非 `main`）
- [ ] `HF_TOKEN` / 私有仓 Authorization
- [ ] 断点续传 / 校验和
- [ ] 仅 `HF_HUB_ENDPOINT` 的文档化矩阵（实现后写进 example）

## 模块边界

```text
src/app/cli/tokenizer.rs          薄：clap + 打印 + 退出码
src/infra/config/types.rs         ModelEntry.tokenizer 字段 + schema
packages/xylitol-ai-bridge
  registry/                       resolve：配置覆盖 + builtin
  tokenize/                       cache + download_opt_in(url) + list/remove
```

- CLI **禁止**直接 `reqwest`
- download 只调 bridge；URL 由 bridge（或共享 helper）根据 `HF_ENDPOINT` 生成
- tokenizer 子命令 **尽量**不强制完整 agent bootstrap；读配置解析 tokenizer 映射可用「只读加载 AppConfig」轻路径（tasks 钉：可复用 resolve_assembly 的配置子集，但 MUST NOT 启会话/MCP）

## 退出码

| 码 | 何时 |
|---|---|
| 0 | 成功；Builtin/Local 无需下载的说明；clean 幂等未找到 |
| 1 | 取消确认；unmapped；下载失败；用法错误 |

## 测试策略

- Bridge：tempdir list/remove；失败不留坏文件；URL 拼装尊重 `HF_ENDPOINT`
- Registry：配置覆盖 / builtin / unmapped
- CLI：`run_with_cache`；help 树含 status/download/clean
- BDD：ce15 场景；paa6 无静默下载仍绿

## 与 c1390 的衔接

c1390 引入 `tui` / `print` 表面动词后：

- 默认裸跑 TUI **不变**
- `tokenizer` **保持**顶层
- 本 design 的模块边界不因 c1390 搬迁
