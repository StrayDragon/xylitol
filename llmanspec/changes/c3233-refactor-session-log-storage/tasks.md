# Tasks — 会话存储重整为追加日志

> 实施者导读：先加后删（新 LogStore 与旧实现并存至 T8 才切换/删除）；每题独立可验证；design § 编号指本文档对应章节。收口走 llman-sdd change finalize，不列为任务。

- [ ] T1 冻结投影等价黄金集（design §3 / §11）
  用旧实现对既有场景（普通对话、fork 后 travel、压缩后加载、分页）的 load_branch_entries 输出抓为 fixture（输入 entry 流 + 期望输出），落 tests/golden/session-log/。验证：生成脚本可重复跑；cargo nextest run -p xylitol-official-plugin-session 绿。
- [ ] T2 log 写路径核心（可与 T1 并行；design §2 / §7 / §9）
  新模块 log_store：session.lock（flock / 信号量，先于一切写创建）、append 一行（seq 分配；Header 行内嵌 format=log-v1）、turn 边界批量 fsync（append_turn / end_turn_flush）、尾部半行截断函数。单测：追加读回、截断、双进程互斥。
- [ ] T3 三投影读路径（design §3）[blocked-by: T1, T2]
  LLM 视图（leaf→root 折叠、Compaction 截断、firstKept 停）、树视图（全量穿透）、确定性属性测试（同 leaf 必同投影）；对照 T1 黄金集全绿。
- [ ] T4 轮转与压缩语义（design §4 / §12）[blocked-by: T2, T3]
  双触发（初版固定 64MB + turn 边界 + 压缩点）、保留尾重发（同 id 字节重发 + newest-file-wins 去重）、manifest 缓存（写入 / 重建 / 丢弃容忍）。单测：崩溃矩阵（append / 轮转 / 压缩 / 重发每步 kill 后可恢复）。
- [ ] T5 锚点索引与分页（design §6 / §12）[blocked-by: T3, T4]
  锚点写入（每 64 行一条）、entry_id 到 (file, offset) 定位 + 无索引回退扫描、page token（v / leaf / anchor / dir / unit / limit）与「最后 N turn」窗口查询。单测：跨文件窗口、token 失效从 leaf 重开。
- [ ] T6 port 接线（design §2；ADR-14）[blocked-by: T5]
  LogSessionStore 实现 XySessionStore 全接口（crates/xylitol-plugin-sdk/src/ports/session.rs:33），配置开关装配（默认仍旧实现）；fork / create / delete / list 走新实现。验证：既有 BDD 场景在双后端开关下均绿。
- [ ] T7 维护单元（design §5）[blocked-by: T4, T5]
  会话打开快检（截尾 / .tmp 清理 / 缓存新鲜度，毫秒级）；xylitol sessions maintain CLI（全量索引重建 / 孤儿清理 / 健康上报 / 阈值建议重算），根包 cli-entry 接线。验证：脏状态注入后打开自愈；CLI 幂等连跑两遍结果一致。
- [ ] T8 兼容门、默认切换与旧码删除（design §8）[blocked-by: T6, T7]
  旧布局（v7 manifest / legacy 单文件）显式 unsupported 文案（见 design §8）；默认后端切 LogStore；删除被替代的旧 manager 路径（persist / seal / segments / load / write_lock 相应部分，保留 port 壳与 edges 语义）。验证：残留符号 rg 清单为空；just fmt / just lint / just test 绿。
- [ ] T9 BDD 与 specs 重写 [blocked-by: T8]
  agent-session-store .feature 受影响 requirement 改写为日志语义（现存 36 条中布局相关部分）；tests/bdd bindings 与场景数据同步更新。验证：llman-sdd validate --all --strict 绿；BDD check_command 全绿。
- [ ] T10 参数基准记录（design §4 / §6）[blocked-by: T7]
  锚点密度 64、自适应阈值公式（中位 turn 字节，范围 [16MB, 256MB]）、fsync 批量成本三项 lab 基准（探针落 scripts/lab_ 前缀）；参数为定值，仅当基准偏离 2 倍以上才回写 design 重议。
