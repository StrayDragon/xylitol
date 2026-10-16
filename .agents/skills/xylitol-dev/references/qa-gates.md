---
brief: "门禁 just qa / just qa-e2e（scripts/check_*.py 接线 / prek 与 qa 正交 / just doctor 自检 / 复杂度门禁 cccc / 元门禁）"
---

# 质量门禁（`just qa` 及其纪律）

命令入口（日常 / 全量 / 真终端 / doctor）见根 `AGENTS.md`「命令与验证」；**组成以 `justfile` 为 SSOT**，本文件只讲纪律与细则，不枚举步骤（枚举会腐化）。

## 门禁纪律（MUST）

1. **非变更门禁脚本 `scripts/check_*.py` MUST 经 wiring 进 `qa`**；维护脚本不进门禁。检查脚本组保持 glob 形态——「写了检查忘了接线」由元门禁（解析 justfile 自身）兜住。
2. **prek 与 `just qa` 正交**：prek hook（任何 stage）只承担**文本 / 元数据卫生**（空白、EOF、私钥、大文件等）与 commit-msg 规约，保持轻量；cargo 级门禁（fmt / clippy / test / doc-test）**MUST NOT** 拆成独立 hook 进 prek。理由：hook 要快，cargo 门禁慢。
3. **push 一致性由 CI 兜底**（GitHub Actions），不用本地 hook 卡推送。LFS pre-push 由 git-lfs 原生 hook 负责（`just setup` 经 `git lfs install --local` 确保安装）。
4. `just qa` 是开发过程中的**手动门禁**，push 前跑不跑靠自觉；`qa` 输出默认 quiet，`just qa normal` / `verbose` 或 `JUST_VERBOSITY=` 调整。

## 环境自检（doctor）

新机器 / 环境异常先 `just doctor`（裸 `just` 会在有环境警告时自动提示）。检查项集中在 `scripts/doctor_dev_env.py` 的 `CHECKS` 表：加一项 = 一个返回 `(ok, detail)` 的探针。

## 真网关 lab（live-provider）

`just qa` 串行跑 `qa-test-live-provider`（连接真实网关的 lab）：配置路径、`enabled` 门禁与 skip 语义见 `justfile` 与 `just gen-live-provider-example`。产品示例配置 `configs/example.yaml` 由 `just gen-config-example` 生成（改 `scripts/gen_config_example.py` 模板，**勿**直接手改产物）。

## 复杂度门禁

- **TUI 端（HARD）**：产品 TUI 入口协调者的函数级复杂度经 `scripts/check_complexity.py`（cccc-rs）强制：Sonar cognitive ≤32 且 McCabe cyclomatic ≤27；测试专用模块不参与。硬门禁组成见 `justfile`。
- **全树雷达（软信号）**：`just complexity`（`--radar`）报告全生产树 cccc-rs top-cognitive 排名，作 review 参照；**不以文件行数**作为质量约束（行数硬测会逼出假拆分）。
- 阈值与豁免细则：`src/AGENTS.md`「复杂度与体量」。

## 元门禁（为什么「检查必被接线」）

`qa-check-scripts-wired` 用脚本解析 `justfile` 自身，强制 `qa` 必须依赖检查脚本组、且检查组必须保持 glob 形态——**给门禁自身上的门禁**。新增检查脚本时无需手工登记，机制上不可能「写了检查忘了接线」。

**从事故沉淀自动化**：新增 `scripts/check_*.py` 时，若源自一次真实事故（而非假设），在脚本 docstring 记录案例，让防护可追溯、可理解。
