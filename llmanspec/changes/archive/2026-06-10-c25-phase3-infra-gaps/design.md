# Design: Phase 3 Infrastructure Gaps

## 1. ModelRegistry 升级

### 当前状态
`ModelRegistry` (50 行) 只是 `Vec<ModelMeta>` 的包装，无 provider 注册、auth 检查、available 列表。

### 设计方案

```rust
struct ProviderConfig {
    name: String,
    api_key: Option<String>,
    base_url: Option<String>,
    priority: u32,       // 排序优先级
    is_oauth: bool,      // 是否 OAuth
}

struct ModelRegistry {
    providers: HashMap<String, ProviderConfig>,
    models: Vec<ModelMeta>,
}

impl ModelRegistry {
    fn register_provider(&mut self, name: &str, config: ProviderConfig);
    fn has_configured_auth(&self, model: &ModelMeta) -> bool;
    fn get_available(&self) -> Vec<&ModelMeta>;  // 按 priority 排序
    fn default_model_id(&self, provider: &str) -> Option<&str>;
}
```

**Key decisions**:
- 不使用 pi 的 API-key-per-provider 动态发现（需要网络）
- Provider 通过配置文件显式注册（aligns with xylitol config system）
- `has_configured_auth` 检查 api_key != None 或 is_oauth

## 2. ModelResolver

### 功能
pi 的 `model-resolver.ts` 提供：
1. 精确匹配：`provider/modelId` 或 bare `id`
2. 模糊匹配：partial id 或 name 匹配
3. 别名优先：alias > dated version
4. `model:thinkingLevel` 解析
5. Fallback model 构建

### 实现

```rust
fn resolve_model(pattern: &str, available: &[ModelMeta]) -> ResolvedModel {
    // 1. Try exact match: "provider/modelId" or bare "id"
    // 2. Try partial match: id contains or name contains
    // 3. Prefer alias (no date suffix) over dated versions
    // 4. Parse "model:thinkingLevel" suffix
}

struct ResolvedModel {
    model: ModelMeta,
    thinking_level: Option<ThinkingLevel>,
    warning: Option<String>,  // e.g., "Using fallback model"
}
```

**Default model per provider** (简化版，仅 OpenAi + Anthropic):
- openai → `gpt-4o`
- anthropic → `claude-sonnet-4-20250514`
- Unknown → first available

## 3. ResourceLoader

### 功能
pi 的 `resource-loader.ts` 负责加载：
- Project context files (AGENTS.md)
- Skills (.xylitol/skills/ or ~/.xylitol/skills/)
- Prompt templates (.xylitol/prompts/ or ~/.xylitol/prompts/)
- System prompt (from CLI/env)

### 实现

```rust
struct ResourceLoader {
    cwd: PathBuf,
    agent_dir: PathBuf,  // ~/.xylitol/
}

impl ResourceLoader {
    fn load_context_files(&self) -> Vec<(String, String)>;
    // 从 cwd 向上到 / 查找 AGENTS.md/CLAUDE.md

    fn load_skills(&self) -> Vec<Skill>;
    // 从 agent_dir/skills/ 和 cwd/.xylitol/skills/ 加载

    fn load_prompt_templates(&self) -> Vec<PromptTemplate>;
    // 从 agent_dir/prompts/ 和 cwd/.xylitol/prompts/ 加载 .md 文件
}
```

**简化**: 不实现 extensions 集成（extendResources, reload with project trust）。这些依赖 Extensions SDK。

## 4. Prompt Templates

### 格式
```markdown
---
description: "Code review a file"
argument-hint: "<file-path>"
---
Review the following file for bugs and style issues:

File: $1

Focus on:
- Logic errors
- Security issues
- Style violations
```

### 参数替换

```rust
fn substitute_args(template: &str, args: &[String]) -> String;
// $1, $2, ... → positional
// $@, $ARGUMENTS → all args joined
// ${N:-default} → positional with default
```

### 加载路径
1. `~/.xylitol/prompts/*.md` (global)
2. `<cwd>/.xylitol/prompts/*.md` (project)
3. CLI `--prompt-path` directories

## 5. Slash Commands

### Built-in commands
```rust
const BUILTIN_COMMANDS: &[(&str, &str)] = &[
    ("model", "Select model"),
    ("compact", "Compact session context"),
    ("session", "Show session info"),
    ("fork", "Fork session at a previous message"),
    ("stats", "Show session statistics"),
    ("new", "Start a new session"),
    ("help", "Show available commands"),
];
```

### Integration in prompt()
```
prompt("text") →
  if text.starts_with("/"):
    if command_match:
      dispatch to command handler
      return
    if template_match:
      expand template → send expanded text to LLM
      return
  // normal flow
```

## 6. OutputAccumulator

### pi 设计
- 接收增量 `Buffer` chunks
- 内存中只保留尾部窗口（rolling buffer）
- 窗口满时打开 temp file，将全部已有数据写入
- `finish()` 返回 snapshot: 尾部内容 + truncation info + temp file path

### 实现

```rust
struct OutputAccumulator {
    max_lines: usize,      // 默认 100
    max_bytes: usize,      // DEFAULT_MAX_BYTES
    max_rolling_bytes: usize, // max_bytes * 2
    rolling_text: String,
    rolling_bytes: usize,
    temp_file: Option<PathBuf>,
    temp_writer: Option<BufWriter<File>>,
    total_bytes: usize,
    all_chunks: Vec<Vec<u8>>, // pre-temp-file chunks
}

impl OutputAccumulator {
    fn append(&mut self, data: &[u8]);
    fn finish(&mut self) -> OutputSnapshot;
}

struct OutputSnapshot {
    content: String,         // truncated tail
    truncation: TruncationResult,
    full_output_path: Option<PathBuf>,
}
```

## 7. SessionCWD 验证

```rust
fn assert_session_cwd_exists(cwd: &str, fallback_cwd: &str) -> Result<()>;
// 检查 cwd 目录是否存在，不存在则尝试 fallback
// 若都不存在，返回包含缺失路径的错误
```

## 8. 集成架构

```
AgentSession
├── ModelRegistry (providers, auth, available)
│   └── ModelResolver (pattern → ResolvedModel)
├── ResourceLoader (context_files, skills, templates)
│   ├── PromptTemplate (expand, substitute)
│   └── SlashCommands (builtin list)
├── SystemPrompt (dynamic build with resource output)
├── EventBus (multi-subscriber)
├── MessageQueue (steer/followUp)
├── RetryState
└── SessionManager
    ├── SessionCWD (validate)
    └── Compaction (LLM summary)
```
