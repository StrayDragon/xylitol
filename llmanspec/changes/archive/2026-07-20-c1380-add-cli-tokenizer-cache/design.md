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

## 配置模型 ↔ 词表（核心 · pre-1.0 从简）

推理 `model` id **经常 ≠** HF 词表仓库（量化后缀、本地路由名等）。配置分开写，并允许 **多模型共用** 同一词表。

### 推荐（共享表 + 短写）

```yaml
tokenizers:
  qwen36:
    repo: Qwen/Qwen3.6-35B-A3B   # → {HF_ENDPOINT}/Qwen/Qwen3.6-35B-A3B/resolve/main/tokenizer.json

models:
  models:
    qwen:
      provider: openai
      model: Qwen3.6-35B-A3B/UD-Q5_K_XL-think-coding   # 推理 id
      base_url: http://tufa:50256/v1
      tokenizer: qwen36                                 # 引用共享词表
```

### 也允许（不建表）

```yaml
tokenizer: Qwen/Qwen3.6-35B-A3B   # 含 / → HF repo
# tokenizer: ./tokenizer.json
# tokenizer: builtin
```

### 解析（dumb，1.0 前可乱）

```text
tokenizer 字符串
  → tokenizers.<name> 存在 → 用其 repo/file 或 path
  → "builtin" → Builtin
  → 像路径 → Local
  → 含 "/" → HuggingFace repo + tokenizer.json
  → 否则错误（不猜）
```

`HF_ENDPOINT` 只影响下载基址，不写进每条 model。

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
