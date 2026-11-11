# Tasks — 上下文注入框架插件（v7 SSOT 对齐）

> 实施者导读：design §1 编译与 SSOT 对照表、§3 配置样张、§4 行样张是验收基准。依赖 c3240 合入。收口走 finalize，不列为任务。

- [x] T1 spec 先行
  agent-prompt 框架 requirement（7 点 SSOT 命名 / 编译 / 三来源 / 排序 / 空注册零开销）+ context_reminder 实例条款；runtime-config context_inject 节；plugin-sdk 助手契约；agent-llm-projection；loop 缝字段；workspace-assembly +1；domain-compaction；domain-security。验证：llman-sdd validate --specs --strict。
- [x] T2 内核数据暴露（唯一 core 改动；依赖 c3240 缝名）
  BeforeStepCtx 增 skills / tools_fingerprint / prepare_fingerprint 三只读字段并供数。单测：与冻结 / prepare 状态对齐。
- [x] T3 sdk 命令助手
  plugin-sdk run_command_capture（spawn + stdout 捕获 + 超时 + host_env 消毒复用）。单测：超时 / 非零退出 / 输出形态。hooks 插件顺路迁移列为后续 change，非本任务。
- [x] T4 插件骨架与协调器
  新 crate（默认装配）；7 点编译（谓词矩阵）；排序确定性；HistoryRow / Ephemeral / Dedup / TTL / group；空注册零开销。单测：编译矩阵 + 排序 + 组合。
- [x] T5 配置注入器
  context_inject 节加载（三层合并、两阶段校验）；经 sdk 助手物化（4KB / 2s / 失败跳过 + warn）。单测：合并 / 校验 / 护栏。
- [x] T6 drift 注入器
  三源指纹；快照 CustomEntry（latest-wins，压缩保留）；turn_pre 边界差分（首评基线 / resume / 每边界）。单测：边界矩阵 + 指纹稳定性。
- [x] T7 注入行与投影
  context_reminder 行（样张 §4）；HistoryRow 投影 user；压缩后快照保留回归。单测 + BDD（skill B→B1；resume）。
- [x] T8 二级扩展口
  ContextInjectorRegistry Ctx 服务 + fake 示例注入器，三步接入验证。单测：注册即入组。
- [x] T9 全量回归 [blocked-by: T1, T2, T3, T4, T5, T6, T7, T8]
  just fmt / just lint / just test；llman-sdd validate --all --strict。
