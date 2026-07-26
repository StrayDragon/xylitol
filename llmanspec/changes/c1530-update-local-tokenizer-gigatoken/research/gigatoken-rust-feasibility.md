# Gigatoken Rust 可行性调研（LocalTokenizer 替换）

调研日：2026-07-26
上游：https://github.com/marcelroed/gigatoken
对照消费点：`packages/xylitol-ai-bridge` → `tokenize::encode_count_at_path`（`tokenizers::Tokenizer::from_file` + `encode`）

## 一句话结论

**blocked（主线）**：无 crates.io；crate 根强制 nightly `portable_simd`；PyO3/numpy/parquet 为硬依赖；xylitol 用 stable。短串 count-only 也吃不到 GB/s 宣传收益。git dep 仅适合独立 nightly spike，不宜替换当前 HF `tokenizers`。

---

## 1. crates.io 是否已发布？

**否。** `GET https://crates.io/api/v1/crates/gigatoken` → **404**（2026-07-26 实测）。

公开交付面是 **PyPI**：`gigatoken` **0.10.0**，上传时间 **2026-07-25T19:15:32Z**（https://pypi.org/pypi/gigatoken/json）。安装文档：`pip install gigatoken`（https://github.com/marcelroed/gigatoken/blob/main/README.md）。

Rust 消费者诉求见开放 issue：

- https://github.com/marcelroed/gigatoken/issues/30 — *Publish the Rust core crate on crates.io with an MSRV*（**open**，2026-07-22；无 maintainer 回复）。正文写明：目前不在 crates.io，须 path/git dep；并建议拆 `gigatoken-core`。

---

## 2. 能否作 Rust 库依赖？crate 元数据

| 项 | 事实 | 来源 |
|---|---|---|
| Cargo package name | `gigatoken` | `Cargo.toml` |
| 版本 | `0.10.0` | 同上 |
| lib name（crate root） | `gigatoken_rs` | `[lib] name = "gigatoken_rs"` |
| crate-type | `["cdylib", "lib"]` | 同上 — **既是 PyO3 扩展也是 rlib** |
| edition | `2024` | 同上 |
| license | **MIT** | `Cargo.toml` + `LICENSE` |
| MSRV | **未声明** | issue #30 在要；`rust-toolchain.toml` 钉 `channel = "nightly"` |
| 建议 git dep | `gigatoken = { git = "https://github.com/marcelroed/gigatoken", package = "gigatoken" }` 后 `use gigatoken_rs::…` | Cargo 惯例 + 上述 `[lib]` |

**硬障碍（相对 xylitol）**

1. `src/lib.rs` 首行 `#![feature(portable_simd)]` → **整 crate 需 nightly**（xylitol：`rust-toolchain.toml` → `stable`）。
2. `pyo3` / `numpy` / `parquet` / `arrow-*` 等为 **非 optional** 依赖（根 `Cargo.toml`）——纯 Rust 计数也会拉 Python 扩展与数据栈。
3. `.cargo/config.toml` 启用 `[unstable] profile-rustflags`（nightly-only）。

旁证：下游 `hermes-tokenizer` 从 Gigatoken 抽出 BPE 核心并去掉 PyO3/nightly/SentencePiece 后才上 crates.io（https://crates.io/crates/hermes-tokenizer；UPSTREAM：https://docs.rs/crate/hermes-tokenizer/latest/source/UPSTREAM.md）。说明 **上游本体尚非「lean Rust lib」产品**。

---

## 3. Rust API：本地 `tokenizer.json` + 短串 encode（无 Python）？

**有路径，但是「库内核心 + 加载器」公开，热路径单文档 encode 多在 PyO3 层。**

### 公开 / 可依赖面（Rust）

来自 `src/lib.rs`：

- `pub use crate::bpe::Tokenizer`（实为 `bpe::tiktoken::Tokenizer`，见 `src/bpe/mod.rs`）
- `pub use crate::batch::{WorkerPool, encode_docs_ragged, sp_encode_docs_ragged}`
- `pub mod load_tokenizer`（`hf` / `hub` / `tiktoken`）
- `pub mod pretokenize`
- `pub use … sentencepiece::EncodeState`

`src/load_tokenizer/hf.rs`（模块注释与 `pub fn`）：

- `HfTokenizer` — `Bpe(Tokenizer)` | `SentencePiece(SentencePieceBPE)`
- `load_hf_slice(&[u8])` — 内存中的 `tokenizer.json`
- `load_hf_bpe(path)` — 路径加载 ByteLevel BPE（无 `byte_fallback`）
- `load_hf_sentencepiece(path)` — `byte_fallback` 的 SP 风格 BPE

`Tokenizer` 关键文档 encode（`src/bpe/tiktoken.rs`）：

- `encode_with_added_tokens` / `encode_with_added_tokens_flat` — 对 `&[u8]` 编码
- `decode` / `vocab_size` / `new` / `new_ranked`

批处理：`encode_docs_ragged` 等（`src/batch.rs`）；单文档辅助 `encode_into` 为 **`pub(crate)`**，外部 crate 应走 `Tokenizer` 上的 `encode_*`。

### PyO3 面（非 xylitol 目标）

同文件：`BPETokenizer::from_hf(PathBuf)`、`encode`、`encode_batch`、`encode_files`；`load_hf_json`；`#[pymodule] gigatoken_rs`。Python 包装：`gigatoken/` + maturin（`pyproject.toml`）。

**对 xylitol**：可无 Python 进程；用 `load_hf_bpe`/`load_hf_slice` + `encode_with_added_tokens_flat(...).len()` 即可覆盖「本地 json + 短串 count」语义——**前提是能编过 nightly + 接受硬依赖**。

---

## 4. 与 HF tokenizers 的 encode 对齐

README（Compatibility Mode）：

> outputs match exactly with what you would get with HuggingFace Tokenizers … at a non-negligible cost to performance.

FAQ bench 带 `--validate`：「validation OK: … documents match」（对照 HF）。

Caveats：

- **兼容模式（Python `.as_hf()`）** 强调字节级/序列一致，但牺牲吞吐；**原生 Gigatoken API** 为 GB/s 路径，README 仍用 validate 对照，但未单独保证「所有 tokenizer.json + 全部 postprocessor」与 HF 全功能等价。
- 加载器明确只支持 **BPE**（含 byte_fallback→SP 引擎）；不支持 WordPiece/Unigram（见下节）。
- PR 例：https://github.com/marcelroed/gigatoken/pull/42（merged）— tiktoken 加载若缺 pretokenizer 上下文曾静默错配 GPT-2；说明「加载路径正确性」仍在修。

对 count-only：需对 registry 实际词表做长度对照；不能默认「git dep 即等于当前 `tokenizers` 0.21 encode」。

---

## 5. Gaps（README Known Issues + 代码）

| Gap | 证据 |
|---|---|
| **WordPiece 未支持** | README Known Issues；`hf.rs` 探测 `WordPiece`/`Unigram` 后 `eyre` 拒绝 |
| **SentencePiece 弱优化** | README：「not nearly as optimized」；bench 最慢行多为 SP 系 |
| **Windows** | README：「Windows has not been tested much, so for now prefer using WSL」 |
| File sinks 未实现 | README Known Issues |
| ABI3 Python 开销 | README Known Issues（与 Rust 库消费无关） |

---

## 6. 对 xylitol LocalTokenizer 的实用判定

现状（本仓）：

- `tokenizers = { version = "0.21", default-features = false, features = ["onig"] }`（`packages/xylitol-ai-bridge/Cargo.toml`）
- `encode_count_at_path`：`Tokenizer::from_file` + `encode(text, false)` → `ids.len()`（`tokenize/mod.rs`）
- 会话级短串、count-only、已缓存 `tokenizer.json`；闸默认 off

| 判据 | 结论 |
|---|---|
| crates.io 正式依赖 | **否** → 不符合「优先等发布」策略 |
| stable 工具链 | **否** → nightly feature |
| 无 Python / 瘦依赖 | **否** → PyO3 硬依赖 |
| API 能力（json 路径 + 短串 count） | **有**（见 §3） |
| 相对 HF 的吞吐收益（短串） | **未证明**；上游 bench 是 ~12GB 文件 / 多核 |

**Verdict: blocked**

理由：git dep 在技术上能 spike，但不满足 xylitol 主线约束（stable、供应链、瘦依赖）；GB/s 价值主张与 session count 错位。应继续观察 #30 lean `gigatoken-core`；或另评已剥离 nightly 的第三方提取（如 `hermes-tokenizer`，**非本调研验收对象**）。

---

## 7. 对照表：HF `tokenizers`（xylitol 现状）vs Gigatoken

| 维度 | HF `tokenizers` crate | Gigatoken |
|---|---|---|
| 交付面 | crates.io（xylitol 钉 `0.21`；最新稳定见 crates.io `tokenizers`，如 0.23.x） | 主：PyPI `gigatoken` 0.10.0；Rust **未**上 crates.io（#30） |
| API 语言 | 纯 Rust 库 | Python-first；Rust = core + PyO3；`crate-type` cdylib+lib |
| 加载 `tokenizer.json` | `Tokenizer::from_file` | Rust：`load_hf_bpe` / `load_hf_slice` / `load_hf_sentencepiece` |
| 短串 encode | `encode(&str, …)` | `Tokenizer::encode_with_added_tokens(_flat)`；Py：`BPETokenizer.encode` |
| 批/文件吞吐 | 中等（多线程 Rust）；非 GB/s 卖点 | 设计目标 GB/s 文件/批处理（README bench） |
| 与 HF 对齐 | 自身即参考实现 | 兼容模式强调 exact；原生路径有 validate，覆盖面有限 |
| License | Apache-2.0（上游 HF tokenizers） | MIT（`LICENSE`） |
| Windows | 广泛使用 | README：少测，建议 WSL |
| 工具链 | stable（xylitol） | nightly + `portable_simd` |

---

## 仓库结构：lib 还是主要为 PyO3？

**两者兼有，产品重心在 Python。**

- 根 crate 同时产出 `lib` 与 `cdylib`；maturin 模块名 `gigatoken.gigatoken_rs`（`pyproject.toml`）。
- `src/lib.rs` 大量 `#[pyclass]` / `#[pymodule]`；`pub mod load_tokenizer` + `bpe::Tokenizer` 是可 Rust 调用的核心。
- `src/main.rs`、`benches/`、Python CLI（`gigatoken bench`）服务吞吐产品，而非「稳定 Rust SDK」。

对 xylitol：**不是**现成的 lean `lib` crate 产品；要用需扛 nightly + PyO3 栈，或等 #30 拆核 / 自抽子集。

---

## 来源清单（主）

- https://github.com/marcelroed/gigatoken/blob/main/README.md
- https://github.com/marcelroed/gigatoken/blob/main/Cargo.toml
- https://github.com/marcelroed/gigatoken/blob/main/src/lib.rs
- https://github.com/marcelroed/gigatoken/blob/main/src/load_tokenizer/hf.rs
- https://github.com/marcelroed/gigatoken/blob/main/src/bpe/tiktoken.rs
- https://github.com/marcelroed/gigatoken/blob/main/rust-toolchain.toml
- https://github.com/marcelroed/gigatoken/blob/main/LICENSE
- https://github.com/marcelroed/gigatoken/issues/30
- https://github.com/marcelroed/gigatoken/pull/42
- https://crates.io/api/v1/crates/gigatoken （404）
- https://pypi.org/pypi/gigatoken/json
- 本仓：`packages/xylitol-ai-bridge/{Cargo.toml,src/tokenize/mod.rs}`、`rust-toolchain.toml`
