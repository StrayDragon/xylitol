---
change_id: c1530-update-local-tokenizer-gigatoken
title: 用 Gigatoken 替换 HuggingFace tokenizers 做本地 token 计数
status: purpose-draft
priority: 1530
depends_on: []
author: agent
---

# c1530-update-local-tokenizer-gigatoken

## Why

LocalTokenizer 档（`packages/xylitol-ai-bridge` → `tokenize::encode_count_at_path`）当前依赖 HuggingFace **`tokenizers` Rust crate**（`Tokenizer::from_file` + `encode`）对缓存/本地 `tokenizer.json` 做精确计数。

[Gigatoken](https://github.com/marcelroed/gigatoken) 宣称对常见 BPE 词表可达相对 HF tokenizers 数百倍～千倍吞吐（SIMD pretokenize + pretoken cache），且兼容模式下可对齐 HF encode 结果。若能作为本仓 LocalTokenizer 后端，可降低：

- 显式 `token_estimate.local_tokenizer: on` 时 footer / compact 阈值估计的本地 encode 成本
- 长 session / 大批量消息估计时的 CPU 占用

**注意**：Gigatoken 公开交付面目前以 **Python 包**（`pip install gigatoken`）为主；仓库内有 **Rust core**，但 **尚未发布 crates.io**（上游 issue [#30](https://github.com/marcelroed/gigatoken/issues/30) 仍 open）。本 change 意向是调研并在可行时接入 **Rust 路径**，不是把 Python 解释器塞进 xylitol 热路径。

## 现状（代码事实）

| 项 | 事实 |
|---|---|
| HF encode 入口 | `tokenize::encode_count_at_path` → `tokenizers::Tokenizer::from_file` |
| 依赖 | `xylitol-ai-bridge`：`tokenizers = { version = "0.21", default-features = false, features = ["onig"] }` |
| Builtin 路径 | 仍用 `tiktoken-rs`（OpenAI cl100k/o200k）；**不在本 change 替换范围** |
| 产品闸 | `token_estimate.local_tokenizer` 默认 `off`（paa10 / rc19）；热路径禁止每 delta 全文 encode（paa3） |
| 缓存 / CLI | `HfTokenizerCache` + `xylitol tokenizer status|download|clean`（paa8 / ce15）保留语义 |

合约锚点：`package-ai-bridge-accounting`（paa6/paa8/paa10）、`runtime-config`（rc18/rc19）、`cli-entry`（ce15）。

## 意向（What Changes）

1. **调研闸**：确认 Gigatoken Rust API 能否从本地 `tokenizer.json`（及现有 cache 布局）做 **count-only encode**；核对 MSRV、license、Windows/Linux/macOS、与本仓 edition/MSRV 兼容性。
2. **后端替换（若可行）**：在 `xylitol-ai-bridge` 的 LocalTokenizer encode 路径用 Gigatoken 替换 `tokenizers` crate；对外仍经现有 registry / cache / config 面，**不**改 Api → RemoteCount → LocalTokenizer → Heuristic 优先级。
3. **对齐验证**：对代表性词表（至少 Qwen / Llama-3 系 / GPT-2 类）做与当前 HF `tokenizers` encode 的 **id 序列或长度** 对照（优先长度一致即可满足计数语义；完整 id 对齐更稳）。
4. **依赖策略**：优先等 crates.io 正式发布；短期可评估 git dependency，但 MUST 文档化 pin 与升级路径；**禁止**为 LocalTokenizer 引入 Python runtime。
5. **回退**：若某词表不支持（WordPiece 未支持、SentencePiece 弱优化等），MUST 降级到现有 HF `tokenizers` 或跳过 LocalTokenizer 档（诚实 provenance），MUST NOT  silently 错数。

## 非目标

- 不改 Builtin（`tiktoken-rs`）路径
- 不改 RemoteCount / Api usage 优先序
- 不默认打开 `local_tokenizer`（仍默认 off）
- 不做 GB 级语料批处理 API（本产品是会话级短文本计数，非训练数据管线）
- 不引入 `pip` / 子进程调用 Python Gigatoken

## 风险与开放问题

| 风险 | 说明 |
|---|---|
| 无 crates.io | 可能只能 git dep；供应链与 CI 可复现性变差 |
| 用例错位 | 上游 bench 是 GB 文件吞吐；本仓是短字符串 count——加速比可能远小于宣传数字，仍可能有绝对延迟收益 |
| 兼容成本 | HF 兼容模式为对齐会牺牲吞吐；需在「快」与「与旧 encode 一致」间取舍 |
| 词表覆盖 | WordPiece 未支持；SP 慢；需列出本仓 registry 实际映射的覆盖率 |
| 缓存格式 | 是否仍只需 `tokenizer.json`，或需额外 merges/vocab 文件 |

## Capabilities（正式化时）

- `package-ai-bridge-accounting`（LocalTokenizer 实现后端）
- 可能触及 `runtime-config` / `cli-entry` 文档措辞（若用户可见后端名变化）

## Impact

- **代码**：主要 `packages/xylitol-ai-bridge`（`tokenize`、`Cargo.toml`）；主仓 `token_estimator` 宜保持 seam 不变
- **依赖**：增 Gigatoken（git 或 crates.io）；视对齐策略决定是否保留 `tokenizers` 作回退
- **行为**：LocalTokenizer `on` 时计数更快；provenance 仍为 LocalTokenizer；默认 off 用户无感
- **测试**：bridge 单测 + 与 HF 对照 fixture；不要求改 BDD 热路径（paa3 不变）

## 正式化前

本文件为 **purpose-draft**。升级完整 propose（tasks + live specs attach）前须澄清：

1. 是否接受 git dependency，或阻塞到 crates.io？
2. 未覆盖词表：硬失败 / 回退 HF / 跳过 LocalTokenizer？
3. 对齐标准：仅 `len(ids)` 还是完整 id 序列？

下一步：用户确认后走 `llman-sdd-propose` 正式化，或先 spike 验证 Rust API 再补 tasks。
