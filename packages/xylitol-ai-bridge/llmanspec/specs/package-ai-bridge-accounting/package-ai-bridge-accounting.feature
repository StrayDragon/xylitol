# language: zh-CN
# capability: package-ai-bridge-accounting
# purpose: "xylitol-ai-bridge 上下文 token 计量：Api 优先，否则 Heuristic。"
# scope: src/

功能: package-ai-bridge-accounting

  @req:r1561
  规则: 计量优先级
    上下文 token 估计 MUST 按固定优先级解析来源：先 Api，否则 Heuristic；结果 MUST 携带 TokenProvenance（或等价枚举）标明实际采用的来源。MUST NOT 再经本地词表 encode 或独立 RemoteCount HTTP 作为估计档。

    场景: estimate-priority-api-first
      假如 会话条目含 stop 回合的 usage 锚点
      当 以同源估计器估计上下文
      那么 估计来源为 Api
  @req:r1564
  规则: Api-锚点规则
    当存在适用于当前会话 prefix/leaf 的可信厂商 usage 时 MUST 优先用作 Api 锚点；因 abort 或 error 结束的回合 usage、以及 compaction 之后不再描述当前 prefix 的旧 usage，MUST NOT 作为锚点。

    场景: aborted-usage-not-anchor
      假如 会话条目仅含 abort 回合的 usage 锚点
      当 以同源估计器估计上下文
      那么 估计来源降级而非 Api
  @req:r1563
  规则: Api-trailing-范围
    当 Api 锚点有效时，trailing_tokens MUST 仅计入 last usage 消息之后的消息（对齐 pi estimateContextTokens）；MUST NOT 对锚点及之前的消息再叠加 heuristic；last_usage_index MUST 为该锚点消息下标；若消息列表中找不到带 usage 的 assistant，可将整段列表视为 trailing 且 last_usage_index 可为 null。trailing 所用启发式 MUST 与无锚点 Heuristic 档同一除数（见 r1929）。
    # verified-by: packages/xylitol-ai-bridge/src/accounting/mod.rs
  @req:r1565
  规则: 禁止流上全量重估
    generate_stream 热路径 MUST NOT 在每个文本 delta 上对累计全文重算完整上下文估计。
  @req:r1566
  规则: Heuristic-诚实标注
    当最终采用 Heuristic 时，ContextTokenEstimate 的 provenance MUST 为 Heuristic；下游展示层若使用该结果 MUST 能区分于 Api。
    # verified-by: llmanspec/specs/domain-compaction/domain-compaction.feature

    场景: estimate-fallback-heuristic
      假如 无可信 XyUsage 锚点
      当 以同源估计器估计上下文
      那么 估计来源为 Heuristic 且不得标为 Api
  @req:r1929
  规则: 启发式除数同源
    无 Api 锚点的 Heuristic 档、Api trailing、切点逐条累计所用启发式、固定请求开销折算与摘要占位 MUST 共用同一校准：对序列化后的 UTF-8 字节长度向上取整除以 3。MUST NOT 再使用除以 4 的平行启发式作为决策或展示度量。
    # verified-by: packages/xylitol-ai-bridge/src/accounting/mod.rs
