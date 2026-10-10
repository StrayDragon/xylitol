---
depends_on: []
needs_specs_change: true
branch: sdd/c2855-update-token-estimate-notice-stack
base_branch: main
base_sha: 7b47d5015ba1915312feb2afbca9858ba8e13d6e
---

# 本地词表下线：Api 优先、启发式 /3，客户端右上角通知栈

## Why

YAML 别名可任意命名，按模型名猜 tiktoken 词表会系统性错数；LocalTokenizer 默认关闭、compact 路径也几乎不走，却拖着 `tiktoken-rs` / HF `tokenizers` / CLI download / 配置面。最准的数字是厂商 `usage`；没有时用偏宽的字节启发式更安全（coding agent 上 `/4` 低估代码约 28%）。

同时，成功类短提示（Copied）挤在下缘 status 旁，拒闸通知条又强制 `Error: ` 前缀——启发式「首次说明」不能走错误槽。产品端需要一条**客户端**堆叠时间线（右上角浮层），承载 Copied 与首次估算提示；侧板展开留后波。

## What Changes

1. **计量优先级**改为 **Api → Heuristic**。去掉 LocalTokenizer 档与产品路径上的 RemoteCount（后者从未打开）。Api trailing 与无锚点整段估计共用同一启发式。
2. **启发式校准**：UTF-8 序列化字节 `div_ceil(3)`（同源常数）。切点走查、固定请求开销、摘要占位、图片占位 MUST 用同一除数，禁止再平行 `/4`。
3. **下线本地词表面**：删除 builtin/HF/path 注册、`xylitol tokenizer` CLI、`tokenizers:` / `models.*.tokenizer` / `token_estimate.local_tokenizer` 配置字段、`tiktoken-rs` 与 `tokenizers` 依赖。旧 YAML 键静默忽略（根 schema 本就不拒未知字段）；**MUST NOT** 加载告警或 warn。
4. **TokenProvenance**：产品面保留 Api / Heuristic / Unknown；去掉 LocalTokenizer 与 RemoteCount。footer：Api 为 `used C tokens`，Heuristic 为 `used ~C tokens`。
5. **客户端通知栈**（TUI）：右上角纵向堆叠（最多约 3 条），自动逐条消失；不进 transcript、不占下缘 dock 行。Copied 与首次 Heuristic 说明走此栈。底部通知条仍只服务拒闸/错误（`Error: `）。本波不做 Tab/回车动作、不做展开侧板。
6. **依赖选型**写入 `.agents/skills/xylitol-dev-candidates`（tiktoken / HF tokenizers / gigatoken / RemoteCount HTTP）：何时再拿出来；一次性迁移步骤不进 skill。delayed-change c1530 视为作废。

## Capabilities

- `package-ai-bridge-accounting`（优先级、启发式除数、删词表/RemoteCount 条款）
- `runtime-config`（删 tokenizer 配置条款）
- `cli-entry`（删 tokenizer 动词；ops 清单）
- `domain-compaction`（估计链、chars/4 字面、切点同源）
- `app-tui-fixed-zone`（footer provenance；通知条 vs 通知栈分工）
- `app-tui-host`（Copied 落点改通知栈）
- `protocol-app` / `server-core`（estimate_context 不再提 host 词表映射）
- `package-ai-bridge`（RemoteCount 作为交付面退出）

## Impact / 风险

- 无 usage 的会话（Fake、首轮）footer `~` 数字变大约 33%（`/4`→`/3`），compact 更早——预期、偏安全。
- 去掉 CLI `tokenizer` 是破坏性命令删除；无数据迁移脚本（缓存目录可残留，产品不再读）。
- 通知栈浮在 transcript 右上，可能短暂遮住品牌/滚动首行；短 TTL + 最多 3 条。
- 线协议 provenance 字符串 `LocalTokenizer`/`RemoteCount` 解码为 Unknown（或忽略），旧客户端展示可能变成 `used ? tokens` 直到升级。

## Further Notes

- 多语言校准见会话调研：英文 `/4` 偏高；代码 `/4` 偏低 ~28%；CJK×o200k 接近；CJK×cl100k 偏低 35%+。`/3` 对代码几乎贴合。
- 设计稿对照：`designing/tui-lab/modules/toast-stack/` 晋级到产品 `designing/tui/modules/toast-stack/`（本波不做键盘动作键）。
- gigatoken 一手调研仍在 `llmanspec/delayed-changes/models/c1530-update-local-tokenizer-gigatoken/research/`，skill 只留指针。
