# hermes-tokenizer 本地 spike（Qwen3.6）

调研日：2026-07-26
crate：[hermes-tokenizer 1.8.98](https://docs.rs/hermes-tokenizer/latest/hermes_tokenizer/)（自称从 GigaToken 抽出、stable Rust、无 PyO3）
词表：`Qwen/Qwen3.6-35B-A3B` `tokenizer.json`（经 `HF_ENDPOINT=https://hf-mirror.com` + `xylitol tokenizer download Qwen/Qwen3.6-35B-A3B --yes`）
宿主：升级后 `rustc 1.97.1`（hermes MSRV **1.97**；升级前本机 1.95 无法编译该 crate）

## 一句话结论

**不能作为本仓 LocalTokenizer 的权宜替换。** 对产品实际使用的 Qwen3.6 `tokenizer.json`，`Tokenizer::from_file` **直接失败**；需改写 merges / 字段 / `use_regex` 才能加载。改写后短串 id 与 **原始** HF encode 在本 spike 探针上对齐，但加载路径已不是「缓存里的那份 json」，且改写会改变 HF 自身对部分输入的 encode——不可静默接入。

## 环境

| 项 | 值 |
|---|---|
| 缓存路径 | `~/.xylitol/tokenizers/Qwen__Qwen3.6-35B-A3B/tokenizer.json`（~12.8 MB） |
| 对照 | `tokenizers` 0.21（与 `xylitol-ai-bridge` 一致）vs `hermes-tokenizer` 1.8.98 |
| Spike | `/tmp/hermes-tokenizer-spike-*`（未进仓库依赖） |

## 原生加载（无改写）

| 文件 | hermes `from_file` |
|---|---|
| Qwen3.6 原文件 | **FAIL**：`merges` 为字符串 `"Ġ Ġ"`，期望 length-2 array |
| 仅 merges→array | **FAIL**：`BPE continuing_subword_prefix is not supported`（字段值为 `""` 也被拒） |
| 再去掉 prefix/suffix | **FAIL**：`ByteLevel pre_tokenizer with use_regex=false is not supported` |
| 镜像 GPT-2 原文件 | **FAIL**：同样 string merges |

Qwen3.6 事实：`model.type=BPE`，`merges` 全为 **str**（新版 HF 序列化），`byte_fallback=false`，`ByteLevel.use_regex=false`，外包 `Sequence(Split+ByteLevel)`。

## 强行改写后（实验性）

改写步骤（仅实验）：merges str→`[left,right]`；删除 `continuing_subword_prefix` / `end_of_word_suffix`；强制 `ByteLevel.use_regex=true`。

| 结果 | 事实 |
|---|---|
| hermes 能否加载 | **能**（vocab≈248070） |
| vs **原始** HF id 序列（12 probes + ~200 条假 session） | **全匹配**（`id_mismatches=0`，session `3290` tokens） |
| **同一改写文件** 上 HF vs 原始 HF | **有差异**（至少 `def foo...` 多行代码探针 `HF_MUTATION_CHANGED`）→ 改写不是语义无感 |
| 计时 40×短串（同机 release） | xylitol 现状模式 `from_file+encode` ≈ **26.9 s**；HF 缓存 encode ≈ **2.6 ms**；hermes 缓存 ≈ **0.98 ms** |

解读：相对「每次 from_file」的巨大差距来自 **重复加载**，不是引擎名；缓存后 hermes 约 2–3× 快于 HF，但未证明值得换依赖。真正低垂果实仍是 **内存缓存已加载 Tokenizer**。

## 与 xylitol 约束对照

| 判据 | 结论 |
|---|---|
| MSRV | 要 **1.97+**（本仓已升 stable 1.97.1 后可编） |
| 直接吃 `HfTokenizerCache` 的 json | **否** |
| count-only 对齐（改写后探针） | 看起来可以，但依赖非官方预处理 |
| 供应链 | crates.io 有；下载量极低、版本号狂飙（1.8.81→1.8.98 数日内），偏实验包 |
| 产品默认 `local_tokenizer: off` | 换引擎用户几乎无感 |

## Verdict

**blocked as drop-in / stopgap for Qwen3.6 LocalTokenizer.**

可选后续（均另开决策，非本 spike 验收）：

1. 继续用 HF `tokenizers`，先做 **Tokenizer 句柄缓存**（性价比最高）。
2. 若仍想 hermes：上游需支持 string merges + `use_regex=false`（或空 `continuing_subword_prefix`），再重跑无改写对照。
3. 等真正的 lean `gigatoken-core` crates.io（c1530 原线）。
