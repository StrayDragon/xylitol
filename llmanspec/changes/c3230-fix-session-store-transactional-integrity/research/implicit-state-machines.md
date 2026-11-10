# 会话存储隐式状态机扫描证据（2026-11-10）

静态阅读结论；未编译、未运行测试。风险分级为扫描时判断，实施前需逐条复核。

## S1 压缩提交多文件事务（高，implicit）

- 位置：plugins/xylitol-official-plugin-session/src/manager/seal.rs:78-127（已复核原文）
- 载体：文件系统 + manifest 的 sealed_segments/active_segment，无事务标记
- 状态集合：旧布局完整 / 冷段已写未挂 / sidecar 已写未挂 / active 已写未挂 / 新布局已挂但旧 active 未删
- 转移点：seal.rs:96-99 写冷段；:101-107 写 sidecar（失败回滚前两项 :105）；:108-114 写新 active（失败回滚三项 :112）；:115-121 写 manifest（失败回滚三项 :119）；:123-124 删旧 active（不可回滚）
- 守卫：每步错误只 remove_paths；seal.rs:174-186 拦绝对路径与 ..
- 缺失：崩溃在 manifest 写成功后、旧 active 删除前 → 磁盘同存两代 active，旧 active 成孤儿且 load 不报错；cleanups 在写 active 前已含 active_path（:100），回滚会删从未写入的文件
- 复核记录：Lead 已读 seal.rs:70-140 原文，转移点与回滚序一致

## S2 段代际计数器 generation（高，implicit）

- 位置：segments.rs:435-444（已复核：sealed_segments 各段 generation ∪ active_segment.generation 取 max，saturating_add(1)）
- 载体：u64，由现存段最大值 +1 推导，非存储计数器
- 状态集合：INITIAL_GENERATION=0（segments.rs:14）到 N；active-{g}.jsonl 与 segments/{g:020}-sealed.jsonl 两套命名
- 转移点：seal.rs:78 压缩封段；segments.rs:228 fork 封段；:499-503 replace_entries；:52-56 current_active_path 解析失败静默回退 0；:435-443 无段时返回 1
- 缺失：读-算-写无锁（replace_entries 在 commit_entries 外无锁）；manifest 不可读时回退 active-0.jsonl 可能命中上一代残留读到旧历史

## S3 会话落盘布局状态（高，implicit）

- 位置：persist.rs:79-109、:240-320（已复核 flush 四支与 append 分支）
- 载体：manifest.json 存在 / {id}.jsonl 存在 / active-*.jsonl 孤儿 / pending_store 内存键，四者组合
- 状态集合：unknown / pending-only（未落盘）/ v7 目录 / legacy 单文件 / orphan active
- 转移点：load.rs:16-22 legacy+无 manifest 报错、无 manifest 读 pending；persist.rs:92-108 flush 四支；:173-194 create 三支；:247-307 append 两支，:304-306 仅 assistant 触发 flush（已复核：is_assistant 判定在 :281 附近，flush_pending_to_disk 仅 assistant 路径调用）；seal.rs:37-46 压缩前补 create + flush；store.rs:75-86 load_done_bash_ids 再判一遍
- 缺失：无单一 layout_of()；非 assistant 的终态写入不落盘是隐式规则

## S4 leaf 锚点优先级链（高，implicit）

- 位置：mod.rs:27、:96-105（已复核 get_leaf：unwrap_or(None) 合并缺键与显式 None）
- 载体：leaf_ids: HashMap<String, Option<String>> 三态 + manifest.leaf_entry_id
- 转移点：mod.rs:97 set_leaf；persist.rs:318-319 append 后前移；persist.rs:145-149；load.rs:42 图分支解析后重写；load.rs:163 分段加载写回 resolved_leaf；load.rs:262-268 load() 用末条覆盖；tree.rs:14-21 branch/reset_leaf；segments.rs:471 与 seal.rs:125 压缩/追加后前移
- 缺失：无单一 resolve_leaf()；fork 分支（load.rs:27-31）顺序为 hint → get_leaf → manifest → anchor，普通分支（load.rs:49-55）为 hint → get_leaf → anchor → manifest，两处回退顺序不同（已复核）
- 复核记录：Lead 已读 load.rs:20-60 原文确认两分支顺序差异

## S5 sidecar 索引可用性（中，implicit）

- 位置：segments.rs:183-195、load.rs:86-97
- 载体：SessionSegment.index_path: Option<String>
- 状态集合：无索引（全段扇扫）/ 有索引（跳段）/ 索引损坏（静默回退扇扫）
- 转移点：segments.rs:92-102 写索引；:104-124 读失败只 warn 返回 None；load.rs:87-97 用索引挑候选段；:115-133 未命中退化全扇；store.rs:91-115 done_bash_ids 同样分支
- 缺失：静默降级掩盖真实损坏；索引与段内容无校验和

## S6 写锁租约（中，implicit）

- 位置：write_lock.rs:50-74
- 载体：write_locks map + 进程级 flock
- 状态集合：未持锁 / 本进程已持 / 他进程持有 / 跳过（无 manifest）
- 转移点：:51-53 in-memory 直接 Ok；:54-56 无 manifest 也返回 Ok（实际不持锁）；:57-60 map 命中返回；:61 真实加锁；:66-68 释放；:82-99 inode 校验重试
- 缺失：「无 manifest 视为已持锁」与随后 create 落盘之间存在窗口，两进程可同时写同一新会话；锁随 map 存活、无 Drop 释放

## S7 fork 子会话判定（中，implicit）

- 位置：tree.rs:190-202
- 载体：manifest.parent + manifest.branches
- 状态集合：普通会话 / fork 子（父段快照）/ fork 子但父段为空
- 转移点：tree.rs:146-150 封父段；:152-153 建子；:189-202 写 parent 与 branches；load.rs:24、segments.rs:276、store.rs:83 三处 parent.is_some() 分流
- 缺失：三处分流无统一谓词；parent.segments 为空时行为等价普通会话但 page_from_segments 走全量分支

## S8 thinking_level / model 折叠（低，implicit）

- 位置：context.rs:49-61；扇扫按物理存储顺序而非图顺序，跳分支变更会污染当前分支；字串非法值无校验透传 provider。

## S9 active_session 死代码（低）

- mod.rs:28；tree.rs:256-266 全部 #[allow(dead_code)]；delete_session 不清空；无清空路径。

## S10 SessionBackend 分支散布（低，explicit-scattered）

- types.rs:8-11；约 15 处 matches!(&self.backend, ...) 各自决定落盘/加锁/读 pending。
