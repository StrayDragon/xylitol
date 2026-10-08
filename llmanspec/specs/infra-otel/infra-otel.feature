# language: zh-CN
# capability: infra-otel
# purpose: 可选 OpenTelemetry OTLP/HTTP 出口：Cargo feature otel + 配置显式开启；默认与失败路径均不收集；经 fastrace-opentelemetry 导出；Langfuse 会话、GenAI 属性、xylitol.obs.lane 与同一 turn 内父子 span 树（含过 prepare 的 agent.compaction）在 opt-in 出口之上叠加。
# scope: src/infra/observability/, src/app/cli/, src/agent/runtime/, src/agent/compaction/
# c2833: packages/xylitol-ai-bridge trace 部分暂留本 cap（verified-by 指向子包源文件）；子包侧拆分待批，scope 行先行收窄（one-path-one-root）。

功能: infra-otel

  @req:r1466
  规则: otel-default-none
    未配置 [otel]、feature 缺省或 exporter=none 时，进程 MUST NOT 向任何 OTLP 端点发送 traces；MUST NOT 仅因存在 fastrace span 而隐式开启远程导出。

    场景: otel-default-none-no-spans
      假如 mock 模型先 tool 后无 tool
      当 以未开启观测闸运行一次带工具调用的 agent 回合
      那么 默认观测闸为关且回合正常完成不出口任何 span
  @req:r1477
  规则: otel-config-opt-in
    当 Cargo feature otel 启用且 AppConfig.[otel] exporter=otlp-http 且 endpoint（及所需认证头）合法时，组合根 MUST 经 fastrace-opentelemetry OpenTelemetryReporter（或等价）将 fastrace span 导出为 OTLP/HTTP；protocol MUST 为 HTTP（binary 或 json），MUST NOT 以 gRPC 作为 Langfuse 目标路径的默认或唯一选项。

    场景: otlp-config-arms-reporter
      当 以合法 otlp-http 配置尝试构建 OTLP reporter
      那么 成功构建出可安装的 reporter
  @req:r1486
  规则: otel-fallback-no-block
    OTLP exporter 构建失败、凭证缺失、feature otel 未启用、或配置不完整时，观测装配 MUST 降级为不安装 OTLP Reporter（本地 file JSONL 闸仍独立），MUST 将诊断写入 file-only 日志（若级别日志已开），MUST NOT 因此使对话主路径启动失败。当 [otel] exporter 请求 otlp-http 而通道未生效（缺 endpoint/认证或构建失败）时，该事实 MUST 经 LoadedResourcesSnapshot.obs_diag 在产品 loaded-resources 卡以单行 obs 诊断呈现；诊断内容 MUST NOT 携带密钥或完整 env；exporter=none 显式关闭时 MUST NOT 渲染提示。

    场景: otlp-bad-endpoint-falls-back-with-diag
      当 以缺失 endpoint 的 otlp-http 配置尝试构建 OTLP reporter
      那么 构建安静返回 None 且产生 obs 诊断且不失败
  @req:r1487
  规则: otel-fastrace-only
    OTLP 出口 MUST 建立在 fastrace Reporter 之上（fastrace-opentelemetry）；Cargo 与 src MUST NOT 为该出口引入 tracing 或 tracing-subscriber；MUST NOT 使用会打印到 stderr/stdout 的 ConsoleReporter 或 OTEL 调试打印破坏 TUI。
    # verified-by: Cargo.toml
    场景: otlp-exit-built-on-fastrace
      当 以合法 otlp-http 配置尝试构建 OTLP reporter
      那么 成功构建出可安装的 reporter

  @req:r1488
  规则: otel-fanout-with-file
    当本地 provider-trace FileReporter 与 OTLP Reporter 同时满足开启条件时，组合根 MUST 使用 fan-out（或等价）同时投递同一批 SpanRecord；任一侧失败 MUST NOT 静默关掉另一侧的既有闸门语义（OTLP 侧失败按 otel3 降级）。
    # verified-by: src/infra/observability/fanout.rs
    场景: file-and-otlp-fan-out
      假如 mock 模型先 tool 后无 tool
当 以观测闸开启、会话 UUID 与收集槽运行一次带工具调用的 agent 回合
      那么 agent.turn 根 span 携带等于会话 UUID 的 langfuse.session.id

  @req:r1489
  规则: otel-session-id-always
    当低频观测 span 激活且当前会话 UUID 已知时，导出根 span（至少 agent.turn；以及无父 turn 时的独立 token.estimate 与独立 agent.compaction）MUST 携带属性 langfuse.session.id，其值 MUST 等于该会话 UUID；同 turn 子 span MUST 继承同一 trace 从而同属该 session；MUST NOT 用 display name 替代 session id。

    场景: otel-session-id-on-turn-root-headless
      假如 mock 模型先 tool 后无 tool
      当 以观测闸开启、会话 UUID 与收集槽运行一次带工具调用的 agent 回合
      那么 agent.turn 根 span 携带等于会话 UUID 的 langfuse.session.id
  @req:r1490
  规则: otel-session-name-optional
    当用户已为当前会话设置 display name 时，上述根 span MUST 额外携带 langfuse.trace.metadata.session_name（值为该 display name）；未设置 name 时 MUST NOT 写入该属性；后期命名 MUST 只影响后续 span，且 MUST NOT 改变 langfuse.session.id。

    场景: otel-session-name-metadata-headless
      假如 mock 模型先 tool 后无 tool
      当 以带 display name 的会话身份运行一次 agent 回合
      那么 根 span 携带 session_name 元数据
      当 以无 name 的会话身份运行一次 agent 回合
      那么 根 span 不写 session_name 属性
  @req:r1491
  规则: otel-langfuse-observation-types
    当低频观测 span 激活时：llm.request MUST 标记 langfuse.observation.type=generation 并携带模型名 langfuse.observation.model.name（MUST NOT 再额外写入等价的 gen_ai.request.model 或裸 model）；agent.turn 与 agent.iteration MUST 标记 type=agent；tool.execute MUST 标记 type=tool；agent.compaction MUST 标记 type=span；默认 MUST NOT 在 observation 上附带完整 prompt/completion 载荷（敏感 I/O 仅可经显式后续档开启）。

    场景: otel-observation-types-headless
      假如 mock 模型先 tool 后无 tool
      当 以观测闸开启运行一次带工具调用的 agent 回合
      那么 agent.span 为 agent 且 tool.execute 为 tool
      并且 不写 gen_ai 等价键
  @req:r1492
  规则: otel-generation-usage
    当低频观测 span 激活且 llm.request 流以带 usage 的 Done 结束时，该 generation span MUST 携带 langfuse.observation.usage_details（JSON：input/output/total，可选非零 cache 字段）；MUST NOT 再额外写入 gen_ai.usage.*（与 usage_details 同映射且 inclusive 语义冲突）；无 usage 时 MUST NOT 伪造零用量属性。
    # verified-by: packages/xylitol-ai-bridge/src/provider/trace.rs
    场景: generation-span-carries-llm-lane
      假如 mock 模型先 tool 后无 tool
当 以观测闸开启运行一次带工具调用的 agent 回合
      那么 全部主路径 span 携带 xylitol.obs.lane=llm


  @req:r1467
  规则: otel-observation-io-tier
    AppConfig.[otel].observation_io 缺省或 none 时 MUST NOT 写入 langfuse.observation.input/output；仅当配置为 truncated 或 full 且 span 闸激活时，llm.request generation MUST 按该档写入 observation I/O；I/O 附着 MUST 在流结束（任意 Done）或 span 提前结束时发生，MUST NOT 仅依赖 Done 携带 usage；MUST NOT 因 observation_io 未配置而改变 otel1 默认不出口语义。

    场景: otel-observation-io-tier-headless
      假如 mock 模型先 tool 后无 tool
      当 以 io=none 的观测闸运行一次带工具调用的 agent 回合
      那么 任何 span 都不带 observation input 或 output
      当 以 io=truncated 的观测闸运行一次带工具调用的 agent 回合
      那么 tool.execute 带参数与结果摘要且 agent.turn 带提示预览
  @req:r1468
  规则: otel-turn-span-tree
    当低频观测 span 激活且发生一次用户触发的 agent 处理时，该次处理 MUST 导出为以 agent.turn 为根的同一 fastrace/OTel trace；agent.iteration、llm.request、tool.execute MUST 作为该根的后代并共享同一 trace_id、经 parent_span_id（或等价）挂接；若该 turn 内实际执行了 compaction，则 agent.compaction MUST 同为该根后代并共享 trace_id；MUST NOT 在活跃 agent.turn 下将上述主路径 span 各自以无关的 SpanContext::random 根 span 导出。

    场景: otel-turn-span-tree-headless
      假如 mock 模型先 tool 后无 tool
      当 以观测闸开启运行一次带工具调用的 agent 回合
      那么 iteration、tool 与 token.estimate 均为 turn 根的后代并共享 trace_id
  @req:r1469
  规则: otel-product-span-names
    当低频观测 span 激活时，OTLP/Langfuse 导出名 MUST 使用产品词汇：agent.turn（根）、agent.iteration、llm.request、tool.execute；若发生 compaction 则 MUST 使用 agent.compaction；MUST NOT 再以 react.stream、react.turn 或 provider.request 作为导出名。

    场景: otel-product-span-names-headless
      假如 mock 模型先 tool 后无 tool
      当 以观测闸开启运行一次带工具调用的 agent 回合
      那么 导出名全部为产品词汇（含 token.estimate）且无旧名
  @req:r1470
  规则: otel-token-estimate-parent
    当低频观测 span 激活时：若存在活跃 agent.turn 上下文，token.estimate MUST 作为该 turn 的子 span；若不存在 turn 上下文（真·闲置路径：换叶/resume/显式刷新等），token.estimate MAY 为独立根 span 且在 session 已知时 MUST 携带同一 langfuse.session.id；MUST NOT 伪造父 turn。对同一次 TurnSettled settlement，MUST 至多导出一个 token.estimate；MUST NOT 在 turn 刚结束后仅为 footer stream-close 兜底再开 SpanContext::random 独立根。

    场景: idle-settlement-independent-root
      假如 观测闸开启且绑定会话身份
      当 以闲置路径结算一次 token 估计
      那么 token.estimate 为独立根且携带会话 id

  @req:r1471
  规则: otel-tool-observation-io-tier
    AppConfig.[otel].tool_observation_io 缺省或 none 时 MUST NOT 在 tool.execute 上写入 langfuse.observation.input/output；仅当配置为 truncated 或 full 且 span 闸激活时，tool.execute MUST 在结果落定后按该档写入参数与结果摘要；MUST NOT 与 observation_io 合并为同一配置键；MUST NOT 因 tool_observation_io 未配置而改变 otel1 默认不出口语义。

    场景: tool-io-tier-truncated-only-tools
      假如 mock 模型先 tool 后无 tool
      当 以 io=none 且 tool_io=truncated 的观测闸运行一次带工具调用的 agent 回合
      那么 tool.execute 带参数与结果摘要而其余 span 无 observation I/O
  @req:r1473
  规则: otel-generation-request-body-input
    当 observation_io 为 truncated 或 full 且 span 闸激活时，llm.request generation 的 langfuse.observation.input MUST 优先来自 adapter 发出前的完整 request JSON（按同档硬顶截断）；MUST NOT 仅依赖名为 response.json、chat.completion.json 或 message.json 的 raw 事件才缓冲请求体；流式路径 MUST 同样可带 input。

    场景: otel-generation-input-adapter-capture-headless
      假如 mock 上游按正常完成脚本回放 Anthropic SSE
      当 以 io=truncated 的观测闸对 mock 上游驱动一次流式生成并耗尽全部事件
      那么 llm.request 携带来自适配器 HTTP 前捕获的观测输入


  @req:r1474
  规则: otel-generation-abort-finalize
    当低频观测 span 激活且 llm.request 在未见成功 Done 的情况下提前结束（用户 abort 或等价 drop 流）时：若 observation_io ≠ none，generation MUST 仍按档 flush 已缓冲的 input/output；该 generation MUST 携带 langfuse.observation.level=ERROR 与 langfuse.observation.status_message=aborted；MUST NOT 因此伪造 usage_details。

    场景: otel-generation-abort-flush-headless
      假如 mock 上游仅回放半程 SSE 后挂起
      当 流式生成收到首个文本增量后放弃该流
      那么 llm.request 以 ERROR/aborted 收口并按档 flush 输入输出且不伪造 usage


  @req:r1475
  规则: otel-parallel-tool-spans
    当低频观测 span 激活且 barrier_parallel 并行窗内并发执行多个工具时，各 tool.execute MUST 仍为同一 agent.iteration（进而同一 agent.turn）的子 span 并共享 trace_id；MUST 携带可区分的 tool_id（及可选 tool_batch.mode / tool_batch.barrier_index）；MUST NOT 为并发工具各自创建无关的 SpanContext::random 根 span；并发任务 MUST 使用扇出前捕获的显式 parent，MUST NOT 依赖全局 parent slot 的竞态读写。由单测或 obs 窄读覆盖，MUST NOT 为静态存在性单独扩 BDD step。
    # verified-by: src/agent/runtime/obs.rs
    场景: parallel-tools-share-iteration-tree
      假如 mock 模型先 tool 后无 tool
当 以观测闸开启运行一次带工具调用的 agent 回合
      那么 iteration、tool 与 token.estimate 均为 turn 根的后代并共享 trace_id

  @req:r1476
  规则: otel-compaction-span
    当低频观测 span 激活且实际执行会话 compaction（manual / threshold / overflow）时，MUST 导出名为 agent.compaction 的 fastrace span，且 langfuse.observation.type MUST 为 span；「实际执行」MUST 定义为已通过 prepare_compaction（或等价门闸）之后的压缩尝试——prepare 早退（无可摘要历史 / Already compacted 等）MUST NOT 导出 agent.compaction；若存在活跃 agent.turn，该 span MUST 为其子 span 并共享 trace_id；若不存在 turn 上下文，MUST 可为独立根且在 session 已知时携带同一 langfuse.session.id；MUST 携带诚实 reason（manual|threshold|overflow）与 xylitol.obs.lane=llm，结束时 MUST 能暴露 will_retry / aborted 或失败 status（或等价属性）；默认 MUST NOT 将压缩摘要全文写入 langfuse.observation.input/output；观测闸关闭时 MUST 为零/近零开销。由单测（CollectingReporter）覆盖，MUST NOT 为静态存在性单独扩 BDD step。MUST NOT 改变 XyEvent CompactionStart/End 的面语义。

    场景: compaction-span-over-prepare
      当 以观测闸开启并触发一次会话压缩
      那么 导出 agent.compaction 且 type 为 span 并携带原因与 obs lane
  @req:r1478
  规则: otel-turn-terminal-status
    当低频观测 span 激活时，agent.turn 结束 MUST 暴露可过滤终态：若该轮因用户 abort（或等价 cancel 令牌）提前结束，MUST 携带 langfuse.observation.level=ERROR 与 langfuse.observation.status_message=aborted（或等价）；若该轮正常完成，MUST NOT 将 turn 标为 ERROR/aborted。本 req 不强制细分模型/工具 error 类（可 follow-up）。由单测覆盖，MUST NOT 为静态存在性单独扩 BDD step。

    场景: turn-normal-completion-not-error
      假如 mock 模型先 tool 后无 tool
      当 以观测闸开启运行一次带工具调用的 agent 回合
      那么 agent.turn 不携带 ERROR 或 aborted 终态
  @req:r1479
  规则: otel-token-estimate-settlement-once
    当低频观测 span 激活且完成一次 TurnSettled（无随后 AfterCompaction 失效）时，导出的 token.estimate span 计数 MUST 为 1 且挂在该 agent.turn 下；MUST NOT 再出现同秒同 tokens 的第二 token.estimate（含独立根）。由 CollectingReporter 或 provider-trace 窄读覆盖，MUST NOT 为静态存在性单独扩 BDD step。

    场景: settlement-single-token-estimate
      假如 mock 模型先 tool 后无 tool
      当 以观测闸开启运行一次带工具调用的 agent 回合
      那么 token.estimate 恰好导出一次
  @req:r1480
  规则: otel-obs-lane-llm
    当低频观测 span 激活时，LLM/agent 主路径导出名 agent.turn、agent.iteration、llm.request、tool.execute、过 prepare 的 agent.compaction、以及 settlement 路径的 token.estimate MUST 携带属性 xylitol.obs.lane=llm，供 Collector 或直连消费端过滤；该属性 MUST NOT 写入 XyEvent / hooks；MUST NOT 用 xylitol.signal 作为同义属性名。直连 Langfuse 时，本属 infra 的门闸早退 MUST NOT 伪装为上述 LLM 语义 span（见 otel19）。由单测覆盖，MUST NOT 为静态存在性单独扩 BDD step。

    场景: otel-obs-lane-llm-headless
      假如 mock 模型先 tool 后无 tool
      当 以观测闸开启运行一次带工具调用的 agent 回合
      那么 全部主路径 span 携带 xylitol.obs.lane=llm
  @req:r1481
  规则: otel-session-id-per-generate
    当低频观测 span 激活且两路（或以上）绑定不同会话 UUID 的处理重叠进行时，各路导出的根 span 与该路 llm.request MUST 携带自己那次处理所绑定会话的 langfuse.session.id；MUST NOT 因共享进程级会话槽而被另一路中途 bind 覆盖。本 req 不改变「值为 xylitol 书签 UUID」的语义。由包内/观测单测覆盖，MUST NOT 单独扩 BDD step。
    # verified-by: src/agent/runtime/obs.rs
    场景: session-id-per-overlapping-processing
      假如 mock 模型先 tool 后无 tool
当 以观测闸开启、会话 UUID 与收集槽运行一次带工具调用的 agent 回合
      那么 agent.turn 根 span 携带等于会话 UUID 的 langfuse.session.id

  @req:r1482
  规则: otel-session-id-whole-tree-per-processing
    当低频观测 span 激活且一次处理（绑定某会话 UUID 的 generate / compaction）导出任意低频观测 span（agent.turn、agent.iteration、llm.request、tool.execute、过 prepare 的 agent.compaction、token.estimate、react.error、tool.error，含 compaction summarizer 发起的 llm.request）时，这些 span 的 langfuse.session.id MUST 全部等于该次处理所绑定会话的书签 UUID；重叠处理下 MUST NOT 在 span 创建时读进程级会话槽，MUST NOT 因他路 bind 或 reader 物化改写槽而串入其它会话 id。无 run 上下文的闲置路径（如 slash compaction）MAY 以槽为回退。本 req 不改变「值为书签 UUID」的语义。由单测覆盖，MUST NOT 单独扩 BDD step。
    # verified-by: src/agent/runtime/obs.rs
    场景: whole-tree-one-session-id
      假如 mock 模型先 tool 后无 tool
当 以观测闸开启、会话 UUID 与收集槽运行一次带工具调用的 agent 回合
      那么 agent.turn 根 span 携带等于会话 UUID 的 langfuse.session.id

  @req:r1483
  规则: otel-obs-slot-default-identity-only
    进程级观测槽 MUST 仅作为「默认身份」供无 options 的闲置回退（remote count、无快照的 HTTP hooks、无 options 的 ProviderRequestTrace）与测试使用；materialized 会话（turn / iteration / tool / compaction / generate）MUST 使用运行时自持的会话事实快照，MUST NOT 在 span 创建时读该槽。bind / switch_session MUST NOT 写该槽（c2843 Phase B 移除 obs_slot_writes 门：读者结构上不触达写入路径），槽仅由显式 writer 事件（rename、会话恢复）更新；读者物化前后槽内容 MUST 不变。无 run 上下文的闲置路径 MAY 以槽为回退。由单测覆盖（switch 不写槽 + 快照归因），MUST NOT 单独扩 BDD step。
    # verified-by: src/app/core/driver/in_process/tests/session.rs
    场景: obs-slot-written-by-writer-path
当 以观测闸开启并触发一次会话压缩
      那么 导出 agent.compaction 且 type 为 span 并携带原因与 obs lane

  @req:r1918
  规则: otel-owner-attributed-collection
    当收集低频观测 span 断言时（SpanCollectScope），MUST 按处理所有者的会话身份（xylitol.session.id）过滤后再断言，MUST NOT 将并行其他观测源（其它会话 / init_logging 式槽更新者）产出的外来 span 纳入断言面；owner 无会话身份时为闲置路径，其 span MAY 不携带 xylitol.session.id。本 req 使 in-process 并行收集确定化（与 r1481/r1482/r1483 的 owner 归因不变式同源）。
    # verified-by: tests/bdd/steps_otel_obs.rs / packages/xylitol-ai-bridge/src/provider/trace.rs
    场景: owner-scoped-otel-assertions
      假如 mock 模型先 tool 后无 tool
当 以观测闸开启、会话 UUID 与收集槽运行一次带工具调用的 agent 回合
      那么 agent.turn 根 span 携带等于会话 UUID 的 langfuse.session.id

  @req:r1484
  规则: otel-session-dual-identity-and-fork-edge
    当低频观测 span 激活且当前 xylitol session UUID 已知时，一次处理导出的低频观测 span（覆盖范围与 otel24 相同）MUST 携带 xylitol.session.id，其值 MUST 等于该 session UUID；langfuse.session.id MUST 等于 xylitol.session.id，MUST NOT 改成发给 LLM 通道的会话身份。xylitol.session.llm_gateway_session_id MUST 仅在该 span 所对应的 LLM 请求实际向通道呈报了会话身份时写出，且值 MUST 等于该次呈报值；未呈报 MUST NOT 写该键；MUST NOT 用 xylitol.session.id 占位。当该 session 由 fork 产生且会话头记录了父 session 与切点条目时 MUST 写出 xylitol.session.parent_session_id 与 xylitol.session.fork_at_entry_id（切点为 fork 时所选条目 id，含 Before 切位时未拷入子会话的那条）；无父或切点未知则 MUST NOT 写对应键。无处理快照的闲置路径 MAY 省略树边两键。由单测覆盖，MUST NOT 单独扩 BDD step。
    # verified-by: src/agent/runtime/obs.rs
    场景: dual-session-identity-keys
      假如 mock 模型先 tool 后无 tool
当 以带 display name 的会话身份运行一次 agent 回合
      那么 根 span 携带 session_name 元数据
      当 以无 name 的会话身份运行一次 agent 回合
      那么 根 span 不写 session_name 属性

  @req:r1485
  规则: otel-compaction-skipped-span
    当低频观测 span 激活且会话 compaction 的 prepare_compaction 门闸早退（无可摘要历史 / Already compacted 等；auto 静默路径与 manual 路径皆然）且会话身份已知时，MUST 导出名为 agent.compaction.skipped 的轻量 span：MUST 携带跳过原因（skip_reason）与该会话的 langfuse.session.id / xylitol.session.id；MUST NOT 使用 agent.compaction 名（otel19 保留予过 prepare 的尝试），MUST NOT 携带 xylitol.obs.lane=llm，MUST NOT 伪装 LLM 语义（otel22）；auto 早退 MUST NOT 因此发出 CompactionEnd；auto 预检 MUST 仅发生在 Settle（或 overflow 错误收尾），MUST NOT 在工具续跑的 iteration 关闭时导出 skipped；存在活跃 agent.turn 时 MAY 为其子 span；无活跃会话（无可附的会话身份）MAY 省略；观测闸关闭时 MUST 为零/近零开销。由单测（CollectingReporter）覆盖，MUST NOT 为静态存在性单独扩 BDD step。
    # verified-by: src/agent/compaction/obs.rs
# re-review(c2835): 复审结论——本 capability 管辖行为不变；仅测试判据改为隔离 LANGFUSE_* env，OTLP 装配与降级语义未动。（2026-09-29）
    场景: skipped-compaction-single-estimate
      当 以观测闸开启并触发一次会话压缩
      那么 导出 agent.compaction 且 type 为 span 并携带原因与 obs lane

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）
