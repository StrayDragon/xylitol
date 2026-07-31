# Tasks: c1220-add-safe-minijinja-system-prompt

## 测试缝（深挖 D13 已对齐）

| ID | Seam | 断言 |
|---|---|---|
| S1 | `build_system_prompt` / 内部 render | 钉 date 时输出稳定；默认路径含 builtins tools / mcp discover（pt11） |
| S2 | 沙箱单测 | strict 未知键失败；include 未知名失败；ctx 无 env/secret |
| S3 | 既有 BDD | assemble / skills / pt9 / pt11 / apply_* 不回归 |
| S4 | pt5 场景 | 删 `no-jinja-dep`；窄正向或 `feature:false` 指单测 |

## 1. Specs landing

- [ ] 1.1 `change start` → `sdd/c1220-add-safe-minijinja-system-prompt`
- [ ] 1.2 改写 live `agent-prompt` pt5 + feature（废 no-jinja-dep）
- [ ] 1.3 `validate` 相关 specs `--strict --no-check`；commit Specs landing

## 2. 沙箱 + 模板

- [ ] 2.1 `agent/prompt` 沙箱 Env 构建（strict、白名单 ctx、预注册 include）[blocked-by: 1.3]
- [ ] 2.2 嵌入入口 + 少量 partials；Rust 过滤 mcp:/skills 可见性后喂 ctx [blocked-by: 2.1]
- [ ] 2.3 `SystemPromptOpts` 可注入 date；门面接线默认路径走 render [blocked-by: 2.2]
- [ ] 2.4 SYSTEM/APPEND/context 纯文本路径保持语义 [blocked-by: 2.3]

## 3. 测试与校验

- [ ] 3.1 单测：S1/S2；跑 S3 相关 BDD [blocked-by: 2.4]
- [ ] 3.2 `llman sdd validate c1220-… --strict` [blocked-by: 3.1]
