---
brief: "门禁 just qa / just qa-e2e（scripts/check_*.py 接线 / 版本命名闸 / prek 与 qa 正交 / 复杂度门禁 cccc / 元门禁）"
---

# 质量门禁（`just qa` 及其纪律）

命令入口（日常 / 全量 / 真终端）见根 `AGENTS.md`「命令与验证」；**组成以 `justfile` 为 SSOT**，本文件只讲纪律与细则，不枚举步骤（枚举会腐化）。

## 门禁纪律（MUST）

1. **非变更门禁脚本 `scripts/check_*.py` MUST 经 wiring 进 `qa`**；维护脚本不进门禁。检查脚本组保持 glob 形态——「写了检查忘了接线」由元门禁（解析 justfile 自身）兜住。
2. **prek 与 `just qa` 正交**：prek hook（任何 stage）只承担**文本 / 元数据卫生**（空白、EOF、私钥、大文件等）与 commit-msg 规约，保持轻量；cargo 级门禁（fmt / clippy / test / doc-test）**MUST NOT** 拆成独立 hook 进 prek。理由：hook 要快，cargo 门禁慢。
3. **push 一致性由 CI 兜底**（GitHub Actions），不用本地 hook 卡推送。LFS pre-push 由 git-lfs 原生 hook 负责（`just setup` 经 `git lfs install --local` 确保安装）。
4. `just qa` 是开发过程中的**手动门禁**，push 前跑不跑靠自觉；`qa` 输出默认 quiet，`just qa normal` / `verbose` 或 `JUST_VERBOSITY=` 调整。

## 环境

新机器先 `just setup`（hooks、LFS、可选 cccc 进 `.tools/`）。缺 `cccc-rs` 时 `qa-check-complexity` 会 `cargo install cccc-rs-cli`（pin 见 justfile）。其它缺工具在第一次触发对应 recipe 时失败，按报错补。产品示例配置 `configs/example.yaml` 由 `just gen-config-example` 生成（改 `scripts/gen_config_example.py` 模板，**勿**直接手改产物）。

## 复杂度门禁

- **TUI 端（HARD，cccc）**：产品 TUI 入口协调者经 `just qa-check-complexity`：**cognitive≤32、cyclomatic≤27**。cargo-crap **不**替代（无 cognitive）。硬门禁见 `justfile`。
- **质量雷达（软，本地）**：`just complexity`（cccc 认知 top）+ `just crap-duplicates` +（有 `lcov.info` 时）`just coverage-lcov` → `just crap`；单包用 `just crap-package <pkg>`（包 coverage + 包报告，无 baseline）。
- **CRAP（CI 回归）**：`just coverage-lcov`（`cargo llvm-cov` **≥0.9.1**，矩阵同 `just test`）→ `just crap`。配置 `.cargo-crap.toml`（排除 generated/harness/tests；`epsilon=0.25` 容忍覆盖率抖动，真实回归通常 ≥1.0）。**不进 `just qa`**。`crap.yaml`：PR **CRAP 分数回归** + 评论；`crap-duplicates.yaml`：结构重复写入 job summary（信息性）。`fail-above=false`（存量超阈不挡合并）。
- **signals（cargo-crap ≥0.10）**：`.cargo-crap.toml` 预留 `[signals]`；发布后用 `just crap-signals` / CI 接线。
- **cccc 是否退役**：临时对比见 `references/complexity-cccc-crap.md`（不进 `scripts/`）；当前结论保留 cccc HARD，CRAP 管覆盖×复杂度与重复。详见 `src/AGENTS.md`「复杂度与体量」。
- 阈值与豁免细则：`src/AGENTS.md`「复杂度与体量」。

## 内环（非门禁）

- **`just test-changed`**：按当前 diff 映射到包 + **直接**反向依赖，跑 nextest；根级文件（`Cargo.toml`、`rust-toolchain.toml`、`tests/`、`llmanspec/`、`scripts/` 等）变更回退 `just test`。只是内环，push/PR 仍跑全量 `just qa`。
- **`just crap-package <pkg>`**：单包 coverage + CRAP 报告；无 baseline，只做本地雷达。CI 仍跑全 workspace 回归。

## E2E CI（非阻塞）

- `.github/workflows/tui-e2e.yaml`：`workflow_dispatch` + nightly + PR 带 `tui-e2e` label；装 tmux 后跑 `cargo test --test tui_e2e -- --ignored --test-threads=1`（每个用例经 `cargo run` 起 Host，并行会争 cargo 锁）。
- **不加入 required checks**；先观察 flaky，再决定是否升级为阻塞。本地仍用 `just qa-e2e`（含 `just qa`）。

## SDD validate 分层

- `just qa` 的 `qa-spec-validate` 用 `llman-sdd validate --all`（非 strict）：硬错误失败；未勾 task / specs-not-landed 等 warning 只显示，不红 in-flight change。
- `just sdd` 与 `llman-sdd change finalize` 仍 `--strict`：tasks-done / specs-landed 是收口完成信号。

## 元门禁（为什么「检查必被接线」）

`qa-check-scripts-wired` 用脚本解析 `justfile` 自身，强制 `qa` 必须依赖检查脚本组、且检查组必须保持 glob 形态——**给门禁自身上的门禁**。新增检查脚本时无需手工登记，机制上不可能「写了检查忘了接线」。

**从事故沉淀自动化**：新增 `scripts/check_*.py` 时，若源自一次真实事故（而非假设），在脚本 docstring 记录案例，让防护可追溯、可理解。

## 版本命名闸（只读）

- **入闸**：`scripts/check_no_version_names.py`，由 `qa-check-scripts` glob 自动接线；只读，无写入模式，违例输出 `file:line`。
- **卡两类**：①禁用字面量（线协议旧载体名及其变体，大小写规则与清单见脚本常量），扫描范围内文件的原始行都算；②`.rs` / `.py` 标识符中的版本段（`_v<digits>` 或 CamelCase 的 `V<digits>`），注释与字符串不参与②。
- **白名单只放外部契约**：`uuid` 的 `new_v4` / `new_v7`、URL 路径（如 `/v1`）、会话落盘版本（`v5`–`v7`）、外部模型名。线协议与产品 API 名不得借白名单豁免。
- **处理**：改为语义名；协议版本只经 `PROTOCOL_VERSION` 与 `host.describe` 协商。清单以脚本为 SSOT，本文件不复述。

## 注释闸（uncomment，只读）

- **策略 SSOT**：根目录 `.uncomment.toml`（AST 识别注释，不碰字符串里的 `//`）。
- **入闸**：`scripts/check_uncomment.py`，由 `qa-check-scripts` glob 自动执行；**仅** `uncomment --check --changed-lines`（相对 `UNCOMMENT_BASE` / `origin/HEAD` / `main`），**禁止**在 `just qa` 里跑删除或 `lint --fix`，避免污染工作区。
- **渐进**：只卡相对 base **有改动的行**；历史行内注释不挡整树。
- **保留**：TODO/FIXME、工具指令（clippy/noqa/…）、`~keep` / `NOTE:` 等 `preserve_patterns`；`///` / docstring 默认可移除（`remove_docs = true`）。追溯以 specs / 提交信息为 SSOT。
- **手工**：全树盘点 `uncomment scan`；批量删除仅本地显式 `uncomment .`（不进 `qa`）。可选 `uncomment lint` 见 `[lint]`（默认 `enabled = false`）。
- **跳过 / CI**：`XYLITOL_SKIP_UNCOMMENT=1`；CI 安装 `uncomment` 且 `checkout` 需 `fetch-depth: 0`（PR merge-base）。
