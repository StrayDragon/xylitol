# activity-fold

折叠减噪：低级操作收进簇；默认最小化入口。

## 词表（屏上）

| 态 | 文案 |
|---|---|
| Thinking | 流式 thinking 簇头；无时长、无 `(Ctrl+T)` |
| Thought / Thought {Ns} | 结束后；有墙钟才写秒数（如 `Thought 17s`） |
| Used N | N=**调用次数**；1 次写短名 |
| Editing / Exploring / Running | 进行时簇头（文件层互斥 Editing vs Exploring；有 shell 才追加 Running） |
| Edited / Explored / Ran | 助手正文封口后过去式 |
| Asking questions | Ask 等待尾行 |
| Worked for {duration} | 旧 turn 信封（缺戳则省略时长，禁止伪造） |
| Working / Running {name} | 状态条 busy 短词 |

**禁止**：`Planning next moves`（簇头、尾行、状态条都不进）。

## 可观察 MUST

- 三角在**行首**：收起 `▸`、展开 `▾`。`Asking questions` **不**画三角；整行可点展开打开簇。
- 默认：打开簇有工具时画带三角簇头；流式工具块是子项，**默认折叠**。无 thinking/Ask/工具时不画假占位尾行。
- 展开后自上而下：统计摘要头（三角）→ 子项细账 → Ask 等待时 `Asking questions` 在最底。
- 仅 thinking 的簇不得同时画 `Thinking`/`Thought` 簇头与 thinking L1 `(Ctrl+T)` 头。与工具同簇时簇头走真实活动（`Used` 等），**不用** `Thought` 当聚合头。
- 助手正文第一个非空白字符封口本簇；助手正文不进簇。ScrollNotice / Error **不**进信封。
- 信封（默认）：已结束 turn 可见 User + `Worked for` + 该 turn **最后一段** Assistant。流式当前 turn 不套信封。
- 信封/簇/块 **同一列**，不用缩进表达层级。
- 高度变化 = 一次揭开/收起；**不**要求逐行滑入。

窄宽态本模块固定态不声明；需要时在 state yaml 写 `cols:` / `model:`。
