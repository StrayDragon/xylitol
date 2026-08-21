# spec 约束层级审计（2026-08-22 扫描）

裁决依据：`llmanspec/AGENTS.md`「spec 约束层级」——requirement MUST 描述产品可观察行为/数据契约；禁止钉路径/文件模块名/类型名/方法归属/迁移清单；例外=分层依赖方向、端口 seam、crate 边界、组合根职责、跨面同源。

## 明确候选（violation，逐条裁决后改写为行为陈述）

| req | 现状问题 | 建议方向 |
|---|---|---|
| agent-session-store sp1 | 钉 `infra::session::SessionManager MUST 实现 protocol::SessionStore` 模块路径；且与 sp2 高度重复（同开头，append/load vs load_context/ap…） | 先对拍两行全文：语义等价→合并为一条 port-seam 陈述（infra 实现须落在 protocol 端口，允许提端口 seam 例外但去模块路径） |
| agent-session-store sp2 | 同上（与 sp1 疑似双写） | 同上 |
| agent-tools t18 | 「ToolSet（自 ToolRegistry 重命名）MUST 留在 agent/」含改名史 + 层内归属；编排终态属组织方向例外边缘 | 去「自 ToolRegistry 重命名」迁移史；保留「构建期终态编排状态在 agent 层」（层方向例外）或降为架构 AGENTS 条款 |
| test-provider-integration cv3 | 钉 `std::process::Command` 实现细节 | 改行为：「! 前缀命令执行 MUST 有 10s 超时并以非零/超时结果呈现」，实现载体不钉 |
| infra-git g1 / infra-process p2 / infra-image i5 等「System MUST 提供 fn(cwd)」族 | 函数签名钉死（find_git_repo/build_shell_env/get_themes…） | 改为能力+可观察返回（「向上遍历定位 repo root 并区分 worktree」「PATH 注入 bin 目录」），fn 名删除；涉及 .feature @req 的同步改 |

## 边缘（borderline，倾向保留，记录理由）

- s1 JSONL 落盘位置 ~/.xylitol/sessions —— 用户可见数据契约，留。
- domain-compaction c13 模块拆分条款 —— 组织方向例外（职责内聚边界），留但可在下次触碰时去掉「而非单一超大文件」行数味表述。
- app-tui-host ath12/14/15/16/23、app-tui-commands atm2/atm8 —— 经 Driver/app::core seam = 端口 seam 例外，留。
- test-qa-gate qg03/06/07/08、test-standards ts02、package-tui-testing tt04/06 —— test-* capability 本体就是闸/harness 机制契约，留。
- package-tui-paste-burst pb02 Instant 注入 —— 可测性契约带理由，留。
- package-tui-autocomplete ac02 fd(1) 依赖 —— 外部工具行为契约，留。

## 执行注意

- 改写涉 `.feature @req` 的（g1/p2 族）须同步场景文本与 bindings 步骤字面量。
- 全部走 SDD 票（precedent：c2405/c2410/c2415），不直接编辑。
