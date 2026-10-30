# language: zh-CN
# purpose: "对话 compaction — n-gram 切点检测、LLM 摘要与 token 估计。"

# ── 文件导览（29 规则）────────────────────────────────────────
#   触发与估计   : token 估算/触发阈值 · force/auto 分流 · stale 防抖 · overflow 识别
#   摘要与切点   : compact 摘要 · 会话/分支/LLM 摘要 · 切点 · 双摘要
#   会话与策略   : compact 条目 · agent 集成 · policy 指纹 · 压后地板诊断
#   （精确检索以 @req/规则标题为准，本导览为非强制快速导航）
# ────────────────────────────────────────────────────────────────

功能: domain-compaction
  背景:
    假如 有一个临时工作目录
    并且 配置了上下文窗口为 100000 的模型

  @req:r1395
  规则: token 估算使用统一 Usage 与多源计量
    System 须基于当前消息与可选 XyUsage 锚点估计上下文占用，并经 xylitol-plugin-sdk accounting（或等价注入端口）按 Api 然后 Heuristic 的优先级解析；须继续使用 canonical XyUsage，不得再引入并行的第二套 usage 结构，不得经本地词表 encode 或独立 RemoteCount HTTP 作为估计档。估计结果 须能暴露 TokenProvenance（或经映射的等价来源标注）供上层区分 Api 与降级估计；token_estimator 内 不得再定义重复的 XyUsage。

    场景: 估算使用优先级
      假如 存在可信 XyUsage 锚点
      当 调用上下文估计
      那么 优先采用 Api 语义且仍返回统一估计结构

    场景: 估算回退启发式
      假如 无可信 XyUsage 锚点
      当 调用上下文估计
      那么 采用 Heuristic 而非静默当作 Api

    场景: 来源可用
      假如 完成一次启发式降级估计
      当 检查估计结果
      那么 带有 Heuristic 来源标注且无重复 XyUsage 定义

  @req:r1406
  规则: 应触发 compact
    当 CompactionSettings.enabled 为 true 且 context_window > 0 时，System 须在 contextTokens > 有效触发阈值时判定 threshold auto 应触发 compaction；有效触发阈值 = max(contextWindow - reserveTokens, 压后地板 + 迟滞带)（reserveTokens 来自 CompactionSettings）；压后地板 = 固定请求开销（与 c16 同源折算）+ 有效保留尾预算（c8 clamp）+ 摘要占位（最新 CompactionEntry.summary 的同源启发式估计，无先前摘要时取保守常量）；迟滞带 = 压后地板的 25%（固定常量，不得引入新配置字段）；固定请求开销未知或未注入时阈值 须退化为 contextWindow - reserveTokens（与 c8 clamp 同款退化纪律）。该地板阈值 须仅约束 threshold auto 路径：手动 force（c17）与 overflow（c22）不得受其约束。enabled 为 false 时 不得因用量触发；该判定所用 contextTokens 须与产品 footer / Driver 只读估计同源（同一 Api 然后 Heuristic 入口），不得另用独立的 message 字符串启发式总和，不得再使用独立的百分比阈值（如 compaction_threshold / usage_ratio）作为触发 SSOT。

    场景大纲: 阈值预留门禁
      假如 会话消息估算使用 <tokens> 个 token
      并且 compaction reserveTokens 为 16384
      并且 compaction enabled 为 <enabled>
      当 调用 shouldCompact
      那么 返回 <expect_compact>

      例子:
        | tokens | enabled | expect_compact |
        | 90000  | true    | true           |
        | 50000  | true    | false          |
        | 90000  | false   | false          |

    场景: 预留触发共享页脚估算
      假如 会话叶上存在可信 Api usage 锚点且 footer 同源估计可用
      当 执行 auto-compact reserve 触发判断
      那么 所用 token 数字与同源估计一致且 不另算独立启发式总和
      并且 触发比较式为占用大于有效触发阈值 max(window 减 reserveTokens, 压后地板 加 迟滞带)

    场景大纲: 下限感知阈值
      假如 配置了上下文窗口为 <window> 的模型
      并且 compaction reserveTokens 为 <reserve>
      并且 compaction keepRecentTokens 为 <keep>
      并且 compaction 固定请求开销为 <overhead> token
      并且 会话消息估算使用 <tokens> 个 token
      当 调用 shouldCompact
      那么 返回 <expect_compact>

      例子:
        | window | reserve | keep  | overhead | tokens | expect_compact |
        | 32768  | 16384   | 20000 | 9000     | 20000  | false          |
        | 32768  | 16384   | 20000 | 9000     | 24000  | true           |

    场景: 自动越过阈值
      假如 compaction enabled 为 true
      并且 同源估计已超过 window 减 reserveTokens
      并且 非 abort 的 assistant 回合刚落定
      当 执行 turn 后 threshold auto 检查
      那么 发生 compaction 且 CompactionStart reason 含 threshold

    场景: 自动低于阈值
      假如 compaction enabled 为 true
      并且 同源估计未超过 window 减 reserveTokens
      并且 非 abort 的 assistant 回合刚落定
      当 执行 turn 后 threshold auto 检查
      那么 不发生 compaction

    场景: 自动关闭无压缩
      假如 compaction enabled 为 false
      并且 同源估计远超窗口
      并且 非 abort 的 assistant 回合刚落定
      当 执行 turn 后 threshold auto 检查
      那么 不发生 compaction

  @req:r1417
  规则: compact 摘要
    Compaction 须将较早消息摘要为 CompactionEntry，同时保留最近 N 轮。

    场景: 保留最近
      假如 会话有 50 个轮次
      当 触发压缩保留最近 10 轮
      那么 前 40 轮被总结为一个 CompactionEntry
      并且 会话中剩余 12 条记录（概要 + 10 轮）

    场景: 摘要
      假如 会话有 50 轮
      当 调用 compact
      那么 前 40 轮被摘要为一个 CompactionEntry

  @req:r1418
  规则: compact 会话
    Compaction 须将 CompactionEntry 持久化到 session 的逻辑条目流并更新会话树；当切点前存在非空可摘要前缀时 须将该前缀 seal 为不可变 cold 段，active 段 须只保留 firstKeptEntryId 起的尾部与新 CompactionEntry，且 manifest 提交前后 须保持可恢复。

    场景: 持久化
      假如 compaction 完成
      当 加载会话
      那么 存在含 summary 与切点的 CompactionEntry

  @req:r1419
  规则: 分支摘要
    导航到会话树分支时，System 须生成分支摘要条目桥接上下文缺口。

    场景: 分支摘要
      假如 用户在树中导航到分支点
      当 生成分支摘要
      那么 摘要描述了被跳过的上下文
      并且 当前上下文是连贯的

    场景: 分支
      假如 用户导航到较早分支点
      当 生成分支摘要
      那么 摘要条目桥接上下文缺口

  @req:r1420
  规则: LLM 摘要
    Compaction 须调用配置的 LLM 生成结构化摘要，格式：## Goal / ## Constraints & Preferences / ## Progress (Done, In Progress, Blocked) / ## Key Decisions / ## Next Steps / ## Critical Context。摘要请求 须经任务级模型解析入口（runtime-model-registry m18）取模型：配置了任务模型条目时 须用该条目构建的独立实例发起摘要；条目解析或构建失败时 须回退当前会话模型且 不得静默换模型——回退经归因标注与每次 compaction 至多一次的通知可观测，manual force 路径同样适用。摘要请求的 thinking 档位缺省 须继承所用模型的对话同款默认档（可调模型=支持集末项，仅 off/不可调=off）；compaction 配置的 thinking_level 覆盖值精确匹配所用模型支持集时 须覆盖；不在支持集时 须回退继承档并可观测，不得静默接受变体或使 compaction 失败。摘要格式骨架不变。

    场景: 生成摘要
      假如 会话有 30 轮 user+assistant 含文件编辑
      当 调用 generate_summary
      那么 响应含 Goal、Progress、Next Steps 节及具体文件路径

    场景: 摘要模型入口
      假如 compaction 配置了任务模型条目且该条目可构建
      当 执行 compact 摘要
      那么 摘要请求使用该条目构建的独立模型实例且非当前会话模型实例

    场景: 摘要模型回退提示
      假如 compaction 配置了任务模型条目且该条目构建失败
      当 执行 compact 摘要
      那么 摘要请求回退当前会话模型
      并且 归因标注 fallback 且通知至多一次

    场景: 摘要思考覆盖出设置
      假如 所用模型支持集为 off 与 high
      并且 compaction thinking_level 覆盖为 max
      当 执行 compact 摘要
      那么 摘要请求 thinking 回退继承档且可观测

  @req:r1421
  规则: 切点
    System 须自最新条目向后累计 token，在约 keepRecent tokens 预算处找最近合法切点；累计所用 token 须与统一 Heuristic 同源（序列化 UTF-8 字节向上取整除以 3）；零贡献条目 须跳过；不得以条目原始 JSON 整包长度另套一套除数作为切点 SSOT；合法切点 须含 user、assistant、bashExecution、custom_message、branch_summary（及等价条目类型）；不得在 toolResult 处切断；切在 turn 中部时 须填 turn_start_index 与 is_split_turn=true，切在 turn-start 时 turn_start_index 须为哨兵（如 -1）且 is_split_turn=false。keepRecent 预算 须先经窗口协调 clamp：有效预算取 min(keepRecentTokens, contextWindow − reserveTokens − 固定请求开销估计)（固定请求开销与 c16 同源折算；contextWindow 未知或为 0、或开销未注入时 clamp 不生效，预算即 keepRecentTokens）；prepare 与 compact 须共用同一 clamp，不得出现保留窗预算不低于「窗口减 reserve」而把全部可摘要历史划入保留侧的退化切点。

    场景: 查找裁切
      假如 会话 50 条共 80000 tokens 且 keepRecent=20000
      当 调用 find_cut_point
      那么 切点索引大致保留最后 20000 tokens 上下文

    场景: 裁切助手
      假如 会话在 keep 预算内最近合法切点落在 assistant 消息
      当 调用 find_cut_point
      那么 切点落在该 assistant 且 is_split_turn 为 true 或 false 依是否 mid-turn 而定

    场景: 从不工具结果
      假如 会话含 toolResult 条目
      当 调用 find_cut_point
      那么 first_kept 永不落在 toolResult 索引

    场景: 保留预算
      假如 keepRecent tokens 预算给定且存在多个合法切点
      当 调用 find_cut_point
      那么 保留侧上下文约等于 keepRecent 预算（最近合法切点）

  @req:r1422
  规则: 迭代摘要
    存在先前 CompactionEntry 时，System 须使用更新 prompt 保留既有结构化信息并合并新进展，而非从头开始。

    场景: 迭代
      假如 先前 CompactionEntry 含 summary，新消息已累积
      当 以 previousSummary 调用 generate_summary
      那么 结果保留先前 Done 项并添加新项

  @req:r1396
  规则: 文件跟踪
    Compaction 摘要 须在摘要消息内工具调用引用的文件后追加 <read-files> 与 <modified-files> XML 标签列表。

    场景: 文件
      假如 消息含工具调用：read a.txt、write b.rs、edit c.py
      当 调用 compact_session
      那么 CompactionEntry summary 以 <read-files>a.txt</read-files> 与 <modified-files>b.rs c.py</modified-files> 结尾

  @req:r1397
  规则: compact 条目
    Compaction 须产出含 summary 字符串、firstKeptEntryId（切点后首条保留条目）、tokensBefore 计数及含 readFiles 与 modifiedFiles 列表的 details 对象的 CompactionEntry；新写入条目 须额外带完整 policy 快照（contextWindow、reserveTokens、keepRecentTokens、estimatorVersion），迁移来的旧条目可带 legacy/unknown 标记但 不得伪造当前 policy。

    场景: 写入入口
      假如 会话正在活跃使用
      当 压缩完成
      那么 会话 JSONL 包含 CompactionEntry
      并且 CompactionEntry 包含 summary 字段
      并且 CompactionEntry 包含 firstKeptEntryId 字段
      并且 CompactionEntry 包含 tokensBefore 字段

    场景: 入口
      假如 compact_session 完成并加载会话
      当 CompactionEntry 存在
      那么 summary 非空、firstKeptEntryId 有效、tokensBefore 为正、details 含文件列表

  @req:r1398
  规则: agent 集成
    System 须提供会话压缩入口（compact_current_session 或等价）：估计上下文用量、找切点、调用 LLM 摘要、写入带 policy 快照的 CompactionEntry、以 manifest/active/cold 逻辑流原子 seal 并重载会话状态。

    场景: 场景agent
      假如 agent 会话消息超阈值
      当 调用 compact_current_session
      那么 CompactionEntry 写入会话，会话状态已重载

  @req:r1399
  规则: 模块拆分
    compaction 子系统 须按职责拆分为内聚模块，覆盖 token 估计、切点检测、文件操作跟踪、LLM 摘要与分支摘要，而非单一超大文件。

    场景: 拆分按职责
      假如 compaction 公共 API 已就绪
      当 分别调用 should_compact、find_cut_point 与 compact_session
      那么 各 API 独立成功且返回预期结构

  @req:r1400
  规则: 单一配置来源
    Compaction 配置 须有且仅有一个 serde 面向类型 XyCompactionSettingsConfig 与一个运行时类型 CompactionSettings（字段 enabled / reserve_tokens / keep_recent_tokens / model / thinking_level；model 为可选任务模型条目对象，thinking_level 为可选档名覆盖）；不得再保留 compaction_threshold 或重复 CompactionConfig 定义。

  @req:r1402
  规则: 触发估计同源展示
    auto-compact 的 reserve 触发决策 须消费与 TUI footer 相同的 ContextTokenEstimate（或等价共享 settlement snapshot）；当存在可信 Api 锚点时触发所用 token 数字 须跟 Api，不得在 footer 已标 Api 时仍用独立 heuristic 触发；派生占用百分比（若展示）不得作为触发 SSOT。该共享估计在 Heuristic 路径（无 Api 锚点）须计入固定请求开销——system prompt 与 tool schemas（取 reload 后最新态）的同源折算；存在 Api 锚点时 不得重复叠加（usage.input 已含全请求）。
    场景: 预留共享页脚估算
      假如 会话叶上存在可信 Api usage 锚点且 footer 同源估计可用
      当 执行 auto-compact reserve 触发判断
      那么 所用 token 数字与同源估计一致且 不另算独立启发式总和

  @req:r1403
  规则: force 与 auto 分流
    手动 force 路径（CompactionOrchestrator::compact 或等价，经 Command::Compact 执行器）须不过 reserve 闸；prepare 与 compact 的会话条目输入 须为当前 leaf 分支路径（对齐 pi getBranch），不得仅以整文件线性 load 作为唯一输入；prepare 无内容时 须返回明确错误：末条已是 CompactionEntry 时等价 Already compacted；空 leaf 时等价 Nothing to compact (empty session)；其余确无可摘要历史（含已在 keep_recent 窗内）时等价 Nothing to compact (no summarizable history beyond keep window)；不得再以 session too small 作为上述有上下文失败的用户可见主串（偏离 pi 同文，见 PI_DELTAS）；不得因切点计量低估（相对 pi estimateTokens）把仍有可摘要历史的会话误判为无可摘要；auto 路径（maybe_auto_compact）在 prepare 失败时 须静默跳过（不抛上述用户错误）。Force 的 CompactionStart.reason 须可区分为 manual；threshold auto 的 reason 须含 threshold 语义。不得让手动入口继续调用 maybe_auto_compact。

    场景: 手动强制绕过预留
      假如 用量未超 reserve 闸但 leaf 分支上有可摘要历史（按 pi 同构切点计量超出 keepRecent）
      当 调用 Driver 或 slash force compact
      那么 仍执行 compaction 或仅在末条已是 CompactionEntry 时返回 Already compacted
      并且 不经 maybe_auto_compact 闸
      并且 不因切点 JSON 低估把仍有可摘要历史误报为 Nothing to compact (no summarizable history beyond keep window) 或 session too small

    场景: 压缩使用叶子分支路径
      假如 会话文件序含旁支 sibling 且当前 leaf 在右支
      当 执行 prepare 或 force compact
      那么 切点与摘要范围仅含 leaf 分支条目且不含左支 sibling

    场景: 原因溢出
      假如 overflow Case1 触发 auto-compact
      当 观察 CompactionStart 与 CompactionEnd
      那么 reason 可区分为 overflow 且与 threshold 或 manual 不同

  @req:r1404
  规则: stale 防抖
    threshold auto 检查 不得使用时间戳不新于最新 CompactionEntry 的 assistant 或 usage 锚点再触发（对齐 pi）；无任何可信 usage/估计时 不得盲目 compact。

    场景: 过期守卫之后压缩
      假如 刚写入 CompactionEntry
      并且 仅有压缩前 assistant usage 可用
      当 立即再执行 threshold auto 检查
      那么 不用压缩前 usage 再触发 compaction

  @req:r1405
  规则: split-turn 双摘要
    当 is_split_turn 为 true 时，System 须分别对 history（boundary 至 turn_start）与 turn-prefix（turn_start 至 first_kept）做 LLM 摘要，再按「historyText + 空行 + --- + 空行 + **Turn Context (split turn):** + 空行 + turnPrefixText」合并写入 CompactionEntry.summary；history 无可摘要内容时 historyText 须为 No prior history.；不得丢弃 turn-prefix 不做摘要。

    场景: 拆分双摘要
      假如 find_cut_point 返回 is_split_turn=true 且 turn_start 与 first_kept 之间有可摘要内容
      当 执行 split-turn compact_session
      那么 CompactionEntry.summary 含 Turn Context (split turn) 合并标记且 turn-prefix 已被摘要

  @req:r1407
  规则: tokens_before 同源估计
    CompactionEntry.tokensBefore 须优先来自对压缩前会话上下文（与 buildSessionContext / estimate_from_session_entries 同源）的估计，不得仅以 boundary 内条目字符串启发式累加作为唯一权威。

    场景: 词元之前
      假如 compact_session 完成
      当 读取 CompactionEntry.tokensBefore
      那么 该值来自压缩前会话上下文同源估计而非仅 boundary 启发式累加

  @req:r1408
  规则: overflow 识别、一次 recovery 与分流
    System 须经单一 is_context_overflow_assistant（或等价）判定上下文溢出：含 usage 输入超 context_window、length/max_tokens 且近零输出填窗、以及 stop_reason=error 时对 error_message 的集中 overflow 模式匹配；须应用 non-overflow 排除（如 rate limit）；不得以散落英文子串匹配作为唯一手段；该判定 须同时供 compaction Case1 与 retry 互斥使用。当 sameModel（assistant 的 provider+model 等于当前模型）且判定 overflow 时，System 须以 CompactionStart.reason 含 overflow 语义执行 auto-compact；默认每个 run/用户回合仅允许一次 compact-and-retry；二次仍 overflow 须失败并给出固定说明文案，不得无限循环；willRetry 须仅在 stop_reason 不是成功 stop 时为 true。重试前错误 assistant 不得留在将送入模型的工作上下文（session 历史可保留）。overflow Case1 须先于 threshold Case2 评估；overflow 路径 须复用 stale 防抖；CompactionEnd 须能暴露 reason=overflow、will_retry 与失败时 error_message；不得将非 overflow 错误吞进 compaction。

    场景: 溢出重试成功
      假如 sameModel 的 assistant 被判定为 context overflow 且 stop_reason 非 stop
      并且 compaction enabled 且尚未做过 overflow recovery
      当 执行 turn 后 overflow 检查
      那么 发生 compaction 且 CompactionStart reason 含 overflow
      并且 工作上下文摘掉错误 assistant 后续跑模型且重试成功

    场景: 溢出一次
      假如 本回合已完成一次 overflow compact-and-retry
      并且 再次出现 sameModel overflow
      当 执行 turn 后 overflow 检查
      那么 不再次 compact 或无限重试
      并且 CompactionEnd 含固定失败说明文案且 will_retry 为 false

    场景: 错误模型
      假如 assistant 的 provider 或 model 与当前模型不同且该 assistant 为 overflow
      当 执行 turn 后 overflow 检查
      那么 不因该旧 overflow 触发 recovery

    场景: 阈值路由之后结算
      假如 compaction enabled 为 true
      并且 同源估计已超过 window 减 reserveTokens
      并且 非 abort 的 assistant 回合刚落定
      当 执行 turn 后 threshold auto 检查
      那么 发生 compaction 且 CompactionStart reason 含 threshold

  @req:r1411
  规则: force 可选 instructions
    手动 force compact（Command::Compact / CompactionOrchestrator::compact）须接受可选 instructions（Option<String> 或等价）；非空时 generate_summary（含 split-turn 的 history 摘要）须在结构化摘要 prompt 上追加「Additional focus:」+ 该文本（对齐 pi customInstructions），不得替换整份 Goal/Constraints 骨架；generate_turn_prefix_summary 不得注入 instructions；仅空白或 None 须视为无 instructions；threshold / overflow auto 路径 须不传 instructions，不得复用上一次 manual 的 instructions。

    场景: 强制指令追加焦点
      当 以附加指令合成摘要请求前缀
      那么 摘要请求含 Additional focus 且骨架保留

  @req:r1412
  规则: compact 输入为 leaf 分支
    prepare_compaction 与 compact_session（及 Orchestrator force/auto/overflow）须仅消费当前 leaf 的分支路径条目（对齐 pi getBranch）；不得把旁支 sibling 条目计入切点或摘要范围。
    场景: 裁切时叶子分支
      假如 会话在 keep 预算内最近合法切点落在 assistant 消息
      当 调用 find_cut_point
      那么 切点落在该 assistant 且 is_split_turn 为 true 或 false 依是否 mid-turn 而定

  @req:r1413
  规则: turn-settlement-once
    当一次 Settle（本轮模型不再续跑工具）收尾做 threshold/overflow 预检时，System 须对该次收尾只产生一份 ContextTokenEstimate settlement（同一 tokens/provenance generation）供 compact 决策与产品 footer 消费；工具续跑的 iteration 关闭 不得产生该 settlement、不得跑该预检；不得让 Agent 预检与 TUI TurnEnd/stream-close 在无上下文失效的情况下各自再跑一遍 estimate 并各自打点；若随后实际执行了 compaction，须经 CompactionEnd（或等价）失效并允许新的 settlement。算数入口仍 须为 estimate_from_session_entries（或同源），不得另立第二套尺子。compaction 成功后 须重载 leaf（含新 CompactionEntry 与回填行）并以「summary 折行 + 保留尾 + 固定请求开销（c16 同源折算）」产出一份 AfterCompaction settlement 占位估计，供 footer 与下一轮 reserve 闸消费；该占位估计 不得伴随任何主动模型请求（重算上下文等下一个用户请求经 build_context_entries 同源机制生效）；resume / 会话激活路径 须以同一机制（含固定开销）重建估计（LeafChanged settlement 或 host 同源 unary）。成功的 CompactionEnd 载荷 须携带 tokens_after 与该 AfterCompaction settlement 同源同值（供压后大小呈现与压后地板诊断判定），压后重载 leaf 失败等无法产出 settlement 的退化路径 须缺省 None（消费端落回无 M 词形）；不得为此扩展 CompactionEntry 持久化形状（live-only）。
    场景: 一估算每结算
      假如 存在可信 XyUsage 锚点
      当 调用上下文估计
      那么 优先采用 Api 语义且仍返回统一估计结构

  @req:r1414
  规则: no-invent-reasoning-after-compact
    Compaction 以 CompactionEntry 摘要替换 firstKept 之前的轨迹后，随后经 project_for_llm 与 Responses 组装的 input 须仅回放仍留在保留消息中的 thinkingSignature；不得为已摘要掉的旧 assistant 轮次发明或恢复 reasoning item / thinkingSignature。由单测或文档场景覆盖，不得单独扩 BDD step。
    场景: 摘要节来自保留分支
      假如 会话有 30 轮 user+assistant 含文件编辑
      当 调用 generate_summary
      那么 响应含 Goal、Progress、Next Steps 节及具体文件路径

  @req:r1415
  规则: 压后地板一次性诊断
    auto 路径（threshold / overflow）compaction 成功且其 AfterCompaction settlement tokens ≥ contextWindow（压后仍无可用窗口，固定开销吃满窗口的退化形态）时，System 须经 CompactionEnd 载荷（notice 或等价）发一条可行动诊断（建议：调低 keepRecentTokens / 调高 contextWindow / 精简工具面），每个会话运行（run）至多一次（对齐 c22 每 run 一次 overflow recovery 的作用域纪律）；manual force 路径 不得发诊断；地板阈值本身不构成诊断条件（地板 + 迟滞 ∈ (window − reserve, window) 的受控频繁模式 不得触发诊断）。
    场景: 下限诊断在至多一次
      假如 compaction 配置了任务模型条目且该条目构建失败
      当 执行 compact 摘要
      那么 摘要请求回退当前会话模型
      并且 归因标注 fallback 且通知至多一次

  @req:r1416
  规则: policy 指纹
    CompactionEntry 的 policy 快照 须记录产生该摘要时的 contextWindow、reserveTokens、keepRecentTokens 与 estimatorVersion；后续 resume、inspect 或诊断 须能区分完整当前快照与迁移 legacy/unknown 标记，不得将缺失快照静默解释为当前配置。

    场景: 策略快照已记录
      假如 会话有 50 个轮次
      当 触发压缩保留最近 10 轮
      当 读取最新压缩条目的 policy 快照
      那么 policy 快照记录窗口保留与估计器版本
      并且 legacy 无快照条目不当作当前配置

  @req:r1919
  规则: turn-end 压缩事件送达顺序
    当回合收尾（threshold / overflow auto 路径）经运行流内压缩且产出 CompactionStart / CompactionEnd 时，run 流消费者（server run loop、remote driver downlink 等任何在 AgentEnd 停止读取的消费方）须收到成对的 Start 与 End，且 CompactionEnd 须先于 AgentEnd 到达；流组合 不得因主通道优先分支在无穿插 await 的收尾（如 script hook bus 缺省）饿死侧通道，静默丢弃 End（症状：TUI 折叠块滞留 Compacting…）。该顺序对任何产生方式（含 summarizer 返回空响应的 fallback 路径）一致成立。
    场景: 回合结束溢出事件之前agent结束
      假如 以 overflow 错误响应模型装配可压缩运行库并预置可压缩历史
      当 提交一次回合并收齐事件名序列
      那么 压缩开始与结束事件均到达且结束先于回合收尾

  @req:r1920
  规则: 摘要响应 reasoning-only 非空
    摘要生成（generate_complete 族，含 turn-prefix 摘要）对模型响应 须同时累积可见文本与推理内容（ThinkingDelta 族）；当可见文本为空但推理内容非空时，须将推理内容作为摘要结果返回，不得判为空响应而走 fallback 占位（症状：tufa reasoning.encrypted_content 模式产出纯推理摘要被替换为 [Turn prefix: N entries]）；仅当文本与推理均空时才 须判空并走 fallback。
    场景: 推理仅摘要不空
      假如 以仅输出推理的摘要模型为压缩绑定并预置历史
      当 触发一次压缩
      那么 摘要非空且采用推理内容而非 fallback 占位

  @req:r1924
  规则: 压缩度量口径一致与校准
    压缩决策链 须对同一份「替换后」请求上下文（`build_context_entries` 输出）使用同一可校准的 token 度量源：切点判定（`prepare_compaction` / `find_cut_point` 累加）、触发判定（r1406 同源纪律）与产品 footer / Driver 只读估计 须收敛到同一估算入口族；切点判定 不得以独立字符串启发式或不同除数作为度量 SSOT（lax 兜底仅限消息反序列化失败，不得作为默认决策度量）。该度量对同一上下文的估算与 provider 实测 input 的系统性偏差 须可解释且可对拍（启发式除数与 fixed_context 叠加与否必须可区分），不得出现「展示显示超窗而压缩判定无可压缩」的长期矛盾（症状：footer 高估超窗、压缩永不触发、每回合重复判定）。

    场景: 压缩裁切指标同一来源
      假如 会话含一批可投影的 user 与 assistant 消息（含长文本与图片）
      当 对该上下文计算切点度量与统一估算的逐条组成
      那么 切点累计等于统一估算逐条分解且非常规字符计数

  @req:r2971
  规则: 状态条目跨压缩保留
    Compaction 须保留最新状态类 CustomEntry：plan_mode、plan_doc（每个 plan_id 最新）、plan_ref、workflow、todo_checkpoint 与 agent_todo。切点 firstKeptEntryId 之前的状态条目 须在 compaction 后重新追加，使 load_leaf_branch 与 build_context_entries 可见；不得因压缩把计划模式重置为 execute，或丢失活动 workflow / 计划文档。

    场景: 压缩后保留 plan 模式
      假如 会话处于 plan 模式且有 50 个轮次
      当 触发压缩保留最近 10 轮
      那么 压缩后会话仍为 plan 模式
