# design — c3365-add-abort-turn-end-stamp

## 结论

用「空 content 的 aborted assistant 消息」做轮次结束墙钟标记，端侧投影为不可见但参与分段的最小条目。

## 备选与取舍

| 方案 | 取舍 |
|---|---|
| 空 aborted assistant 标记（采纳） | 复用既有消息形状与 project_for_llm 的 aborted skip；print / LLM 上下文零影响；resume 可直接恢复双端墙钟。 |
| 用相邻 user 消息墙钟补末端 | 违反「不得伪造时长」；轮间隔可达数十秒，误差不可接受。 |
| 新增独立 turnEnd 条目类型 | 需动会话格式与版本演进面；对单一时间戳而言过重。 |

## 关键决定

1. 标记触发条件：turn 以 abort 终止（所有 abort 终止路径，含流中取消与流前取消），
   且本轮未落盘任何 assistant 消息。已有部分 assistant 落盘的轮次 不得 追加标记
   （避免 resume 出现重复 Aborted 通知；其时长恢复维持现状）。
2. 标记形状：content 为空、stop_reason=aborted、无 errorMessage、墙钟取落盘时刻。
   端侧与 LLM 投影对「aborted」消息的既有 skip 规则天然兼容。
3. 端侧投影：标记不得画助手气泡（现行 assistant_terminal_ui 已按空文本跳过气泡），
   也不得画 Aborted 通知（空标记无 error_message 时跳过终止通知），
   但须投影为一个可被分段识别的不可见条目——否则信封无法形成。
   分段规则沿用「一簇中间活动全部为 Compaction 时不得再画簇摘要头」的先例：
   仅含标记的簇不画簇摘要头；折叠信封 须为 User 加 Worked for 行。
4. live 一致性：标记随 AgentEnd messages 到达端侧，live 重建须与 resume 投影同构（r1353）。
   live 当前轮仍不套信封（att33 不变）。
5. 旧会话兼容：无标记的旧 abort 轮次维持「缺任一端省略时长」，不迁移、不伪造。