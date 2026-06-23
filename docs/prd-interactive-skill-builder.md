# PRD: Interactive Skill Builder — 交互式技能构建器

> 状态: Idea v1
> 优先级: P3 (探索性)
> 预计工作量: 2-3 周
> 依赖: TUI PRD 完成，Skills 系统 (infra-skills)

## 1. 概述

### 1.1 愿景

为 xylitol 实现**交互式技能构建器**，让用户能够通过可视化界面创建、编辑、测试和管理 agent 技能（Skills），无需手动编写配置文件。

### 1.2 核心价值

| 场景 | 当前痛点 | 解决方案 |
|------|----------|----------|
| 创建新技能 | 需要手写 YAML/Markdown 配置 | 可视化表单 + 实时预览 |
| 调试技能 | 无法看到技能的执行过程 | 交互式测试 + 逐步执行 |
| 技能组合 | 难以理解技能间的依赖关系 | 可视化依赖图 |
| 技能分享 | 缺少标准化的分享机制 | 技能导出 + 社区市场 |

### 1.3 灵感来源

- **GitHub Actions** 的工作流编辑器
- **Scratch** 的可视化编程
- **VS Code** 的扩展管理器
- **npm** 的包管理

## 2. 功能设计

### 2.1 技能类型

#### 基础技能 (Basic Skill)
```yaml
name: code-review
description: 自动代码审查
trigger: on_file_save
tools:
  - read
  - grep
prompt: |
  审查以下代码:
  {{file_content}}

  关注:
  1. 代码风格
  2. 潜在 bug
  3. 性能问题
```

#### 工作流技能 (Workflow Skill)
```yaml
name: full-refactor
description: 完整重构流程
steps:
  - name: analyze
    skill: code-analysis
    output: analysis_result

  - name: plan
    skill: refactor-planning
    input: analysis_result
    output: refactor_plan

  - name: execute
    skill: code-modifier
    input: refactor_plan
    output: modified_files

  - name: test
    skill: test-runner
    input: modified_files
    output: test_results
```

#### 触发器技能 (Trigger Skill)
```yaml
name: auto-commit
description: 自动提交修改
trigger:
  type: file_change
  pattern: "*.py"
  debounce: 5s
action:
  - git add {{changed_files}}
  - git commit -m "Auto-commit: {{summary}}"
```

### 2.2 可视化编辑器

#### 2.2.1 表单模式 (Form Mode)
```
┌─────────────────────────────────────────────┐
│ Skill Builder: code-review                  │
├─────────────────────────────────────────────┤
│                                             │
│  Name: [code-review                    ]    │
│  Description: [自动代码审查              ]   │
│  Trigger: [on_file_save ▼]                  │
│                                             │
│  Tools:                                     │
│  ☑ read    ☑ grep    ☐ write    ☐ bash     │
│                                             │
│  Prompt Template:                           │
│  ┌─────────────────────────────────────┐   │
│  │ 审查以下代码:                        │   │
│  │ {{file_content}}                     │   │
│  │                                      │   │
│  │ 关注:                                │   │
│  │ 1. 代码风格                          │   │
│  │ 2. 潜在 bug                          │   │
│  │ 3. 性能问题                          │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  Variables:                                 │
│  + file_content (auto)                      │
│  + custom_var (manual)                      │
│                                             │
│  [Preview] [Test] [Save] [Cancel]           │
└─────────────────────────────────────────────┘
```

#### 2.2.2 流程图模式 (Flow Mode)
```
┌─────────────────────────────────────────────┐
│ Skill Builder: full-refactor (Flow)         │
├─────────────────────────────────────────────┤
│                                             │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐ │
│  │ Analyze │───▶│  Plan   │───▶│ Execute │ │
│  │         │    │         │    │         │ │
│  │ Input:  │    │ Input:  │    │ Input:  │ │
│  │ file    │    │ analysis│    │ plan    │ │
│  │         │    │         │    │         │ │
│  │ Output: │    │ Output: │    │ Output: │ │
│  │ analysis│    │ plan    │    │ files   │ │
│  └─────────┘    └─────────┘    └─────────┘ │
│                                      │      │
│                                      ▼      │
│                               ┌─────────┐  │
│                               │  Test   │  │
│                               │         │  │
│                               │ Input:  │  │
│                               │ files   │  │
│                               │         │  │
│                               │ Output: │  │
│                               │ results │  │
│                               └─────────┘  │
│                                             │
│  [+ Step] [Connect] [Delete] [Save]         │
└─────────────────────────────────────────────┘
```

#### 2.2.3 代码模式 (Code Mode)
```
┌─────────────────────────────────────────────┐
│ Skill Builder: code-review (Code)           │
├─────────────────────────────────────────────┤
│ 1 │ name: code-review                       │
│ 2 │ description: 自动代码审查                │
│ 3 │ trigger: on_file_save                   │
│ 4 │ tools:                                  │
│ 5 │   - read                                │
│ 6 │   - grep                                │
│ 7 │ prompt: |                               │
│ 8 │   审查以下代码:                          │
│ 9 │   {{file_content}}                      │
│10 │                                         │
│11 │   关注:                                 │
│12 │   1. 代码风格                           │
│13 │   2. 潜在 bug                           │
│14 │   3. 性能问题                           │
│   │                                         │
│   │ [Form] [Flow] [Preview] [Test] [Save]   │
└─────────────────────────────────────────────┘
```

### 2.3 交互式测试

#### 2.3.1 测试面板
```
┌─────────────────────────────────────────────┐
│ Test: code-review                           │
├─────────────────────────────────────────────┤
│                                             │
│  Test Input:                                │
│  ┌─────────────────────────────────────┐   │
│  │ def calculate_sum(numbers):         │   │
│  │     total = 0                       │   │
│  │     for n in numbers:               │   │
│  │         total += n                   │   │
│  │     return total                    │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  Variables:                                 │
│  file_content: [auto-filled from above]     │
│                                             │
│  [▶ Run Test] [Step Through] [Clear]        │
│                                             │
│  Output:                                    │
│  ┌─────────────────────────────────────┐   │
│  │ 代码审查结果:                        │   │
│  │                                      │   │
│  │ 1. 代码风格:                         │   │
│  │    - 函数名清晰 ✓                    │   │
│  │    - 有类型提示建议                   │   │
│  │                                      │   │
│  │ 2. 潜在问题:                         │   │
│  │    - 缺少输入验证                    │   │
│  │    - 未处理空列表                    │   │
│  │                                      │   │
│  │ 3. 性能:                             │   │
│  │    - 可以使用 sum() 函数             │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

#### 2.3.2 逐步执行
```
Step 1/3: Loading tools
  ✓ read tool loaded
  ✓ grep tool loaded

Step 2/3: Processing prompt
  Template rendered: 456 chars
  Variables substituted: file_content

Step 3/3: Executing
  Agent thinking...
  Tool call: read (preview)
  Response generated: 892 chars

[Next Step] [Continue to End] [Stop]
```

### 2.4 技能管理

#### 2.4.1 技能库
```
┌─────────────────────────────────────────────┐
│ Skill Library                               │
├─────────────────────────────────────────────┤
│                                             │
│  Search: [____________________________]     │
│                                             │
│  Categories:                                │
│  [All] [Code] [Git] [Test] [Docs] [Custom]  │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │ ★ code-review      v1.2.0          │   │
│  │   自动代码审查                       │   │
│  │   Tools: read, grep                 │   │
│  │   [Edit] [Test] [Export] [Delete]   │   │
│  ├─────────────────────────────────────┤   │
│  │ ★ full-refactor    v0.9.0          │   │
│  │   完整重构流程                       │   │
│  │   Steps: 4                          │   │
│  │   [Edit] [Test] [Export] [Delete]   │   │
│  ├─────────────────────────────────────┤   │
│  │   auto-commit      v1.0.0          │   │
│  │   自动提交修改                       │   │
│  │   Trigger: file_change              │   │
│  │   [Edit] [Test] [Export] [Delete]   │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  [+ New Skill] [Import] [Community]         │
└─────────────────────────────────────────────┘
```

#### 2.4.2 技能依赖图
```
┌─────────────────────────────────────────────┐
│ Skill Dependencies                          │
├─────────────────────────────────────────────┤
│                                             │
│  code-analysis                              │
│       │                                     │
│       ▼                                     │
│  refactor-planning                          │
│       │                                     │
│       ├──▶ code-modifier                    │
│       │         │                           │
│       │         ▼                           │
│       │    test-runner                      │
│       │                                     │
│       └──▶ documentation-generator          │
│                                             │
│  [Zoom In] [Zoom Out] [Export SVG]          │
└─────────────────────────────────────────────┘
```

## 3. 技术架构

### 3.1 技能数据模型

```rust
struct Skill {
    id: String,
    name: String,
    version: String,
    description: String,
    category: SkillCategory,
    trigger: Option<Trigger>,
    tools: Vec<ToolRef>,
    prompt_template: String,
    variables: Vec<Variable>,
    steps: Option<Vec<Step>>,  // 工作流技能
    metadata: SkillMetadata,
}

struct Variable {
    name: String,
    var_type: VariableType,
    description: String,
    default_value: Option<Value>,
    required: bool,
}

enum VariableType {
    String,
    Number,
    Boolean,
    File,
    Directory,
    Code,
    Custom(String),
}

struct Step {
    name: String,
    skill_ref: String,
    inputs: HashMap<String, Value>,
    outputs: HashMap<String, String>,
    condition: Option<String>,
}
```

### 3.2 编辑器引擎

```rust
struct SkillEditor {
    // 当前编辑的技能
    skill: Skill,
    // 编辑模式
    mode: EditorMode,
    // 表单状态
    form_state: FormState,
    // 流程图状态
    flow_state: FlowState,
    // 代码状态
    code_state: CodeState,
    // 测试状态
    test_state: TestState,
}

enum EditorMode {
    Form,
    Flow,
    Code,
    Test,
}

impl SkillEditor {
    fn switch_mode(&mut self, mode: EditorMode) {
        // 同步不同模式的状态
        match mode {
            EditorMode::Form => self.sync_to_form(),
            EditorMode::Flow => self.sync_to_flow(),
            EditorMode::Code => self.sync_to_code(),
            EditorMode::Test => self.sync_to_test(),
        }
        self.mode = mode;
    }

    fn validate(&self) -> Vec<ValidationError> {
        // 验证技能配置
        let mut errors = Vec::new();

        if self.skill.name.is_empty() {
            errors.push(ValidationError::MissingName);
        }

        if self.skill.prompt_template.is_empty() {
            errors.push(ValidationError::MissingPrompt);
        }

        // 检查变量引用
        let used_vars = self.extract_variables();
        for var in &self.skill.variables {
            if !used_vars.contains(&var.name) && var.required {
                errors.push(ValidationError::UnusedVariable(var.name.clone()));
            }
        }

        errors
    }

    fn generate_skill_file(&self) -> String {
        // 生成技能配置文件
        serde_yaml::to_string(&self.skill).unwrap()
    }
}
```

### 3.3 测试引擎

```rust
struct SkillTester {
    skill: Skill,
    test_inputs: HashMap<String, Value>,
    execution_trace: Vec<ExecutionStep>,
    current_step: usize,
}

impl SkillTester {
    fn run_test(&mut self) -> TestResult {
        // 1. 验证输入
        let validation = self.validate_inputs();
        if !validation.is_valid() {
            return TestResult::ValidationError(validation);
        }

        // 2. 渲染模板
        let rendered = self.render_template();

        // 3. 执行 agent
        let agent_result = self.execute_agent(rendered);

        // 4. 收集结果
        TestResult::Success(agent_result)
    }

    fn step_through(&mut self) -> Option<ExecutionStep> {
        if self.current_step < self.execution_trace.len() {
            let step = self.execution_trace[self.current_step].clone();
            self.current_step += 1;
            Some(step)
        } else {
            None
        }
    }
}
```

### 3.4 技能市场

```rust
struct SkillMarketplace {
    // 本地技能
    local_skills: Vec<Skill>,
    // 远程技能
    remote_index: SkillIndex,
    // 已安装的技能
    installed: HashMap<String, InstalledSkill>,
}

impl SkillMarketplace {
    fn search(&self, query: &str) -> Vec<SkillSearchResult> {
        // 搜索本地和远程技能
        let mut results = Vec::new();

        // 本地搜索
        for skill in &self.local_skills {
            if skill.matches_query(query) {
                results.push(SkillSearchResult::Local(skill.clone()));
            }
        }

        // 远程搜索
        let remote_results = self.remote_index.search(query);
        results.extend(remote_results.into_iter().map(SkillSearchResult::Remote));

        results
    }

    fn install(&mut self, skill_id: &str) -> Result<()> {
        // 下载并安装技能
        let skill = self.remote_index.download(skill_id)?;
        self.installed.insert(skill_id.to_string(), InstalledSkill {
            skill,
            installed_at: Utc::now(),
            auto_update: true,
        });
        Ok(())
    }

    fn export(&self, skill_id: &str, format: ExportFormat) -> Result<String> {
        // 导出技能
        let skill = self.get_skill(skill_id)?;
        match format {
            ExportFormat::Yaml => Ok(serde_yaml::to_string(&skill)?),
            ExportFormat::Json => Ok(serde_json::to_string_pretty(&skill)?),
            ExportFormat::Bundle => self.create_bundle(skill),
        }
    }
}
```

## 4. TUI 集成

### 4.1 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+S` | 打开技能构建器 |
| `Ctrl+Shift+S` | 打开技能库 |
| `Tab` | 切换编辑模式 |
| `Ctrl+T` | 运行测试 |
| `Ctrl+Shift+T` | 逐步测试 |
| `Ctrl+E` | 导出技能 |
| `Ctrl+I` | 导入技能 |
| `Ctrl+D` | 查看依赖图 |

### 4.2 命令行接口

```bash
# 打开技能构建器
xylitol skill create

# 编辑现有技能
xylitol skill edit code-review

# 测试技能
xylitol skill test code-review

# 列出所有技能
xylitol skill list

# 导出技能
xylitol skill export code-review --format yaml

# 导入技能
xylitol skill import ./my-skill.yaml

# 搜索社区技能
xylitol skill search "code review"

# 安装社区技能
xylitol skill install community/code-review-pro
```

## 5. 使用场景

### 场景 1: 创建代码审查技能
```
用户: 我想创建一个自动审查 Python 代码的技能

步骤:
1. xylitol skill create
2. 填写基本信息:
   - Name: python-code-review
   - Description: 自动审查 Python 代码
   - Category: Code
3. 选择工具: read, grep
4. 编写提示模板
5. 添加变量: file_content (auto)
6. 测试技能
7. 保存并导出
```

### 场景 2: 创建重构工作流
```
用户: 我想创建一个完整的重构流程

步骤:
1. xylitol skill create --type workflow
2. 添加步骤:
   - Step 1: code-analysis
   - Step 2: refactor-planning
   - Step 3: code-modifier
   - Step 4: test-runner
3. 连接步骤的输入输出
4. 测试完整流程
5. 保存
```

### 场景 3: 分享技能给团队
```
用户: 我想把这个技能分享给团队

步骤:
1. xylitol skill export my-skill --format bundle
2. 生成 my-skill.xylitol-skill
3. 团队成员:
   xylitol skill import my-skill.xylitol-skill
4. 或者发布到社区:
   xylitol skill publish my-skill
```

## 6. 实施阶段

### Phase 1: 基础编辑器 (1 周)
- [ ] 技能数据模型
- [ ] 表单模式编辑器
- [ ] 基础验证
- [ ] YAML 生成

### Phase 2: 流程图编辑器 (3 天)
- [ ] 流程图可视化
- [ ] 步骤连接
- [ ] 输入输出映射

### Phase 3: 测试系统 (3 天)
- [ ] 测试面板
- [ ] 逐步执行
- [ ] 结果展示

### Phase 4: 技能管理 (3 天)
- [ ] 技能库界面
- [ ] 搜索和过滤
- [ ] 导入导出

### Phase 5: 社区市场 (持续)
- [ ] 远程技能索引
- [ ] 安装和更新
- [ ] 发布机制

## 7. 竞品分析

| 特性 | xylitol Skill Builder | GitHub Actions | VS Code Tasks | npm |
|------|----------------------|----------------|---------------|-----|
| 可视化编辑 | ✓ | ✓ | ✗ | ✗ |
| 流程图 | ✓ | ✓ | ✗ | ✗ |
| 交互测试 | ✓ | ✗ | ✗ | ✗ |
| AI 专用 | ✓ | ✗ | ✗ | ✗ |
| 终端集成 | ✓ | ✗ | ✗ | ✗ |
| 社区市场 | ✓ | ✓ | ✓ | ✓ |

## 8. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 复杂性 | 编辑器过于复杂 | 渐进式披露，只显示必要选项 |
| 兼容性 | 技能格式不兼容 | 严格的版本控制和迁移机制 |
| 安全性 | 恶意技能 | 沙箱执行 + 权限控制 |
| 性能 | 大型工作流卡动 | 懒加载 + 分页 |

## 9. 成功指标

| 指标 | 目标 | 衡量方式 |
|------|------|----------|
| 技能创建时间 | <5 分钟 | 用户测试 |
| 测试成功率 | >90% | 自动化测试 |
| 用户满意度 | >4/5 | 用户调研 |
| 社区技能数量 | >100 | 统计 |

## 10. 开放问题

1. **技能版本管理**: 如何处理技能的版本升级和兼容性？
2. **权限模型**: 不同技能应该有不同的权限吗？
3. **性能优化**: 大型工作流如何优化执行性能？
4. **协作编辑**: 如何支持多人协作编辑技能？

## 11. 参考资料

- [GitHub Actions](https://github.com/features/actions) - 工作流编辑器
- [Scratch](https://scratch.mit.edu/) - 可视化编程
- [VS Code Extensions](https://code.visualstudio.com/api) - 扩展管理
- [npm](https://www.npmjs.com/) - 包管理
