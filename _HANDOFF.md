# Handoff: c250-fix-config-model-loading 已完成

## 现状

- **c84-refactor-tui-diff-engine**: 已归档(pi 式差分渲染方向巩固,premature-agent 修复)
- **c82-fix-tui-core**: 仍 active(剩 3 个人工 task:冒烟测试/qa/strict;代码部分已完成)
- **c250-fix-config-model-loading**: 代码与 spec 已完成,strict 校验通过,待归档

> 注:c84 和 c250 的代码任务均已完成,详见各自 `tasks.md`。

## c250 修复的配置 bug(用户报告:"默认还是 gpt5.4")

三个缺陷叠加已修复:

1. **兜底占位符**(`registry.rs:90`):`gpt-5.4`/`claude-opus-4-8` → 真实模型
   `gpt-4o`/`claude-sonnet-4-20250514`,附回归测试断言。
2. **默认模型不生效**(`cli/mod.rs` Step 5):启动时未传 `--model` 时,现在用
   `resolve_default_profile().model_config.model` 自动选(profile.model > execution.model
   > models.default_model)。修复前配置里的默认模型从不被使用。
3. **静默吞错**(`cli/mod.rs` Step 2):config 存在但加载到 0 个 model 时发
   `Warning` 指向 `model:`/`models:` 拼写问题,不再静默兜底。

### 用户真实配置状态

`<project>/.xylitol/config.local.yaml` 顶层仍是 `model:`(单数,拼错)。修复后行为:
- 现在:打印 `Warning: config file present but loaded 0 models ...` + 兜底 gpt-4o
- 用户手动把 `model:` 改成 `models:` 后:正确加载 qwen(`--list-models` 显示 qwen)

**建议告知用户**:把 `.xylitol/config.local.yaml` 第 3 行 `model:` 改为 `models:`。

## 已知遗留(future 候选)

1. **本地无鉴权 provider 强制要 API key**:`resolve_api_key` 对 `ModelKind::OpenAi`
   强制 `OPENAI_API_KEY`;指向 `http://tufa:50256/v1` 的本地 provider 本不需 key。
   当前因环境有 key 暂不阻塞,留 future。
2. **composer 的 `ratatui-textarea`**(c84 future):与裸终端理念有张力,自实现
   pi 式 editor 是独立工程。
3. **`deny_unknown_fields`**(c250 future):本次用 ce2 的运行时警告替代;全局加
   会破坏 `#[serde(default)]` 前向兼容。
4. **SettingsManager 与 AppConfig 并行**:两套配置系统待对齐,是大重构。

## 代码入口(c250)

| 文件 | 改动 |
|------|------|
| `src/agent/model/registry.rs` | 兜底默认值常量 + 测试断言 |
| `src/interface/cli/mod.rs` | Step 2 加载校验警告 + Step 3 复用 resolved_profile + Step 5 自动选模型 |

## 验证

```bash
cargo build
cargo test --lib -- model::registry       # 11 条
cargo test --test bdd -- --test-threads=1  # 82 条
just fmt && just lint && just test          # 632 条

# 手动验证
cat > /tmp/cfg.yaml <<'EOF'
models:
  default_model: qwen
  models:
    qwen: { provider: openai, model: x, base_url: http://tufa:50256/v1 }
agents:
  default_profile: default
  profiles:
    default: { model: qwen }
EOF
./target/debug/xylitol --config /tmp/cfg.yaml --list-models   # 显示 qwen
# 拼错 key:model: → Warning + 兜底
```
