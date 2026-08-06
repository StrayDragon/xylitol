# Tasks: c1925-update-responses-thinking-replay-flavor

> Specs landing 须在 `change start` / attach 之后。规划壳可先在默认分支补齐。

## 0. Review 门

- [x] 0.1 主目标 = JSONL→Responses 保真；pi 形 SSOT；不迁 Codex `ResponseItem` 史
- [x] 0.2 回填挂点：`ResponsesStreamState` + `completed`/`incomplete`；二次 `ThinkingEnd`
- [x] 0.3 非法 sig → omit+diagnostic；空 encrypted 仍回放；compaction「不假装旧 signature」
- [x] 0.4 **回放策略 = 唯一默认全量回放**；无 Strip/BestEffort/旋钮
- [x] 0.5 Lab：Ornith medium 761 / off 747 resume cache；写入 research §5.2
- [ ] 0.6 测试 seam：包内 golden + 合成 SSE；不强制新 BDD step

## 1. Specs landing（Branch binding 后）

- [x] 1.1 `package-ai-bridge`：pab15/16 保真+回填；pab25 全量 only；pab26 回填 SSE
- [x] 1.2 **跳过** 任何 `reasoning_replay` / 回放模式枚举 req（pab20 显式禁止）
- [x] 1.3 复核 `pab15`/`pab16` 与保真/回填对齐
- [x] 1.4 compaction c27：摘要后不得假装仍有旧 signature
- [x] 1.5 `agent-runtime`：依赖既有二次 ThinkingEnd + 桥合约（本波不另加 ar）
- [x] 1.6 **跳过** runtime-config YAML 新字段

## 2. 保真重建（apply）

- [ ] 2.1 Assembler / `convert_messages_to_input_items` 全量回放 golden（JSONL 形 → input）
- [ ] 2.2 非法 signature omit + diagnostic/trace

## 3. SSE encrypted 回填（apply）

- [ ] 3.1 `ResponsesStreamState` 按 reasoning `id` 索引 done 项
- [ ] 3.2 `response.completed` + `incomplete`：合并非空 encrypted → 再发 `ThinkingEnd` → 再 `Done`
- [ ] 3.3 合成 SSE 单测
- [ ] 3.4 确认 ReAct 二次签在 MessageEnd 前生效

## 4. 文档 / lab

- [x] 4.1 research §5.2：保真 + lab 数字；策略=唯一全量回放
- [ ] 4.2 敏感说明：JSONL 可含 encrypted 整包
- [x] 4.3 维护 example `lab_resume_prompt_cache`（无 strip 臂；不进 qa）

## 5. 校验

- [x] 5.1 Branch binding：`change start`（`sdd/c1925-…`）
- [ ] 5.2 `llman sdd validate … --strict --no-interactive` → `readyToImplement=true`
- [ ] 5.3 apply 后：`cargo test -p xylitol-ai-bridge` + fmt/lint 触及面
- [ ] 5.4 verify：保真 golden + 回填 SSE；不宣称全 compat 可回放
