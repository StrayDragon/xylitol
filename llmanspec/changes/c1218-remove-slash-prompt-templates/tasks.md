# Tasks: c1218-remove-slash-prompt-templates

## 测试缝（已对齐 · S1–S5）

| ID | Seam | 断言方向 |
|---|---|---|
| S1 | BDD `agent-prompt` | 无 pt3/pt4 场景；其余 pt* 仍绿 |
| S2 | BDD `agent-session` | 无 a21/a22 / `template-dispatch`；slash 命令分发仍绿 |
| S3 | BDD `runtime-resource-discovery` | list/reload **不含** prompts；skills/themes/SYSTEM 仍绿 |
| S4 | 单测 / 编译 | 无 `register_prompt_commands`；loader 不扫 `prompts/`；`get_commands` 无 `template:` |
| S5 | CLI `resources` | 列表/详情无 prompts 段（改现有 harness 或等价步骤） |

## 1. Specs landing（Branch binding 后）

- [x] 1.1 `change start` → 分支 `sdd/c1218-remove-slash-prompt-templates`
- [x] 1.2 改写 live：`agent-prompt`（pt3 改写；删 pt4）；`agent-session`（删 a21/a22；改 a23–a26）；`runtime-resource-discovery`；`runtime-config` rc14；`test-standards` ts03；atc18 措辞
- [x] 1.3 窄合约：pt3 / rd1 / a24 否定场景
- [x] 1.4 `validate` 相关 live specs `--strict --no-check`；commit Specs landing

## 2. 实现删除（垂直：发现 → 注册 → CLI）

- [ ] 2.1 停 `ResourceLoader` prompts 发现与 `PromptTemplate` protocol 类型；改 rd 相关单测 [blocked-by: 1.4]
- [ ] 2.2 删 `agent/prompt/templates.rs`、`register_prompt_commands`、bootstrap 接线；`get_commands` 无 `template:` [blocked-by: 2.1]
- [ ] 2.3 CLI `resources` 去掉 prompts 列表/详情；Settings.`prompts` 字段与加载 [blocked-by: 2.1]
- [ ] 2.4 清理 BDD step（含 `expand_template_body` / template 场景）对齐 S1–S5 [blocked-by: 2.2, 2.3]

## 3. 校验

- [ ] 3.1 `cargo test --test bdd`（相关 feature）+ 受影响单测 [blocked-by: 2.4]
- [ ] 3.2 `llman sdd validate c1218-… --strict`（可带 `--check`）[blocked-by: 3.1]
