# Tasks

## T1 — 计量链：Api → Heuristic `/3`

- [x] accounting 优先级只留 Api / Heuristic；去掉 tokenizer_estimate 与 allow_remote 产品入口。
- [x] 引入同源 `HEURISTIC_BYTES_PER_TOKEN = 3`；heuristic、切点投影、fixed overhead、摘要占位、图片占位、lax 兜底全部改用。
- [x] `TokenProvenance` 去掉 `LocalTokenizer` / `RemoteCount`；wire 旧字符串解码为 Unknown。
- [x] footer 文案：Api → `used C`；Heuristic → `used ~C`；删 LocalTokenizer/RemoteCount 臂。
- [x] 校验：accounting / token_estimator / cut_detector 单测按 `/3` 更新后绿。

## T2 — [blocked-by: T1] 拆本地词表实现与依赖

- [x] 删除 bridge `tokenize` 模块、registry 词表映射、`HfTokenizerCache`、CLI `tokenizer` 动词。
- [x] 删除 config：`tokenizers` 表、`TokenEstimateConfig`、`ModelEntry.tokenizer` 及解析函数。
- [x] `xylitol-ai-bridge` 去掉 `tiktoken-rs`、`tokenizers`、仅服务于缓存路径的 `dirs`（若无他用）。
- [x] `scripts/gen_config_example.py` 去掉词表段；重跑生成 example.yaml。
- [x] **不**加旧键 warn。
- [x] 校验：相关单测删除或改断言；`cargo test -p xylitol-ai-bridge` 绿。

## T3 — [blocked-by: T1] 客户端通知栈 + Copied + 首次 Heuristic

- [x] TUI 右上角通知栈：最多 3 条、TTL 消失、成功 `✓` / 说明 `◆`、不进 transcript、不占 dock 行。
- [x] Copied 迁入通知栈；不再画在 status 旁；MUST NOT 变 `Error: ` 通知条。
- [x] Info 类原 toast（非 Error）改走通知栈；Error 通知条保持 atc22。
- [x] 整段 Heuristic 首次出现时推一条说明（进程内一次）；Api trailing 不弹。Print 不弹。
- [x] 晋级 `designing/tui-lab/modules/toast-stack` → `designing/tui/modules/toast-stack`（本波去掉键盘动作承诺）；词汇表加「通知栈」。
- [x] 校验：ath31 Copied 场景、footer Heuristic `~`、harness 首次提示只出现一次。

## T4 — [blocked-by: T2] BDD / 文档 / skill

- [x] 删或改 BDD：tokenizer CLI、local_tokenizer 闸、builtin tiktoken 表、estimate-fallback-chain 走 LocalTokenizer；ops help 不再列 tokenizer。
- [x] 文档：`压缩与上下文.md`、`配置与档案.md`、bridge `AGENTS.md`。
- [x] skill `xylitol-dev-candidates` 增「已退役本地词表 / RemoteCount」表（tiktoken、HF tokenizers、gigatoken、count_tokens HTTP）；指针到 c1530 research；根 AGENTS Skills 行可补检索词。
- [x] 校验：`cargo test --all-features --test bdd` 相对本分支绿；`just doc-check`。

## T5 — [blocked-by: T3, T4] 门禁

- [x] `just lint`；`just qa` 在 change 分支相对 merge-base 全绿（live-provider 既有 skip）。

不在本任务列 finalize。
