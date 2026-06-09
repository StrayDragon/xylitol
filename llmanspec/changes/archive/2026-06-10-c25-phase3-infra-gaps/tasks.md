# Tasks: Phase 3 Infrastructure Gaps

## 阶段 1: ModelRegistry 升级 + ModelResolver

- [x] T1: `src/agent/registry.rs` — ProviderConfig 结构体 + register/has_auth/get_available
- [x] T2: `src/agent/registry.rs` — 默认模型 ID per provider (openai→gpt-4o, anthropic→claude-sonnet-4-20250514)
- [x] T3: `src/agent/resolver.rs` — resolve_model(pattern, available) → ResolvedModel
- [x] T4: `src/agent/resolver.rs` — 精确匹配: provider/modelId + bare id
- [x] T5: `src/agent/resolver.rs` — 模糊匹配: partial id/name, alias 优先
- [x] T6: `src/agent/resolver.rs` — model:thinkingLevel 后缀解析
- [x] T7: `src/agent/resolver.rs` — build_fallback_model (provider + requested id → base model)
- [x] T8: 单元测试: register/auth/available + resolve_exact + resolve_fuzzy + fallback

## 阶段 2: ResourceLoader

- [x] T9: `src/infra/resource.rs` — ResourceLoader: load_context_files (AGENTS.md 从 cwd 向上)
- [x] T10: `src/infra/resource.rs` — load_prompt_templates (global + project .xylitol/prompts/*.md)
- [x] T11: `src/infra/resource.rs` — load_skills 整合已有 SkillManager
- [x] T12: 单元测试: context_files from ancestor dirs + templates from global/project

## 阶段 3: PromptTemplate + SlashCommands

- [x] T13: `src/agent/templates.rs` — PromptTemplate struct + substitute_args
- [x] T14: `src/agent/templates.rs` — expand 逻辑: /template:name args
- [x] T15: `src/agent/commands.rs` — SlashCommandInfo + BUILTIN_COMMANDS
- [x] T16: `src/agent/session.rs` — prompt() 拦截 / 前缀：commands + templates
- [x] T17: 单元测试: template expand + slash dispatch

## 阶段 4: OutputAccumulator

- [x] T18: `src/agent/tools/accumulator.rs` — OutputAccumulator: append + rolling + temp file
- [x] T19: `src/agent/tools/accumulator.rs` — OutputSnapshot with truncation info
- [x] T20: `src/agent/tools/bash.rs` — 集成 OutputAccumulator（替换 String 拼接）
- [x] T21: 单元测试: small output + overflow + temp file content

## 阶段 5: SessionCWD + Defaults + Diagnostics

- [x] T22: `src/infra/session/manager.rs` — assert_session_cwd_exists(load)
- [x] T23: `src/agent/defaults.rs` — DEFAULT_THINKING_LEVEL, DEFAULT_MAX_ITERATIONS, DEFAULT_COMPACTION_THRESHOLD
- [x] T24: `src/agent/diagnostics.rs` — Diagnostic struct (info/warning/error) + collection
- [x] T25: 单元测试: CWD validation + defaults + diagnostic collection

## 阶段 6: BDD 覆盖 + 集成

- [x] T26: BDD: model registry + resolver 场景
- [x] T27: BDD: prompt template + slash command 场景
- [x] T28: BDD: OutputAccumulator 场景
- [x] T29: BDD: CWD validation + diagnostics 场景
- [x] T30: `just qa` 全绿 (fmt + clippy + test + doc + prek)
- [x] T31: `llman sdd validate c25-phase3-infra-gaps --no-interactive` pass

## 验收标准

- [x] 270+ tests pass (329: 252 lib + 77 BDD)
- [x] ModelRegistry: register_provider, has_configured_auth, get_available, defaults
- [x] ModelResolver: exact, fuzzy, alias-pref, thinking suffix, fallback
- [x] ResourceLoader: context_files from AGENTS.md, templates from .xylitol/prompts/
- [x] PromptTemplate: /template:name args expansion with $1..$N, $@, ${N:-default}
- [x] SlashCommands: builtin command table, / prefix interception
- [x] OutputAccumulator: rolling buffer, temp file, snapshot
- [x] SessionCWD: assert exists on load
- [x] Diagnostics: warning on missing API key, error on missing CWD
