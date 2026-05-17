# c35-add-repeat-detection — Design

## Context

- PRD: §10（自动重复检测与中断恢复）、§10.3（流式检测算法）、§10.4（中断推理机制）、§10.5（恢复策略链）
- 依赖关系见 proposal.md frontmatter（depends_on / blocks 为 SSOT）

## Goals / Non-Goals

### Goals

- 实现流式重复检测中间件（RepeatDetector）
- 滑动窗口 + n-gram HashSet 实时检测
- 中断推理机制（cancel_generation）
- 恢复管理器（alter_prompt / switch_model / adjust_params / delegate_to_planner）
- 配置驱动（YAML）

### Non-Goals

- 不修改模型供应插件接口（以中间件装饰器形式挂载）
- 不实现布隆过滤器优化（HashSet 先够用）
- 不实现 parallel 恢复策略（仅 sequential）
- 不处理本地引擎特有的中断 API（由 adk-model 统一 cancel_generation 抽象）

## Decisions

### Decision 1: 重复检测算法核心

```mermaid
flowchart TD
    TOKEN["接收新 token"] --> WINDOW["加入滑动窗口<br/>VecDeque（长度 W）"]
    WINDOW --> NGRAM["提取 (min_n..=max_n) 后缀<br/>生成 n-gram 集合"]
    NGRAM --> CHECK{"n-gram 在<br/>HashSet 中?"}

    CHECK -->|命中| HIT["consecutive_hits++<br/>repeat_tokens += n"]
    CHECK -->|未命中| MISS["consecutive_hits = 0<br/>插入 n-gram 到集合"]

    HIT --> THRESHOLD{"consecutive_hits<br/>> threshold?"}
    THRESHOLD -->|yes| TRIGGER["触发循环判定"]
    THRESHOLD -->|no| RATIO{"窗口内重复占比<br/>repeat_tokens/W > ratio?"}
    RATIO -->|yes| TRIGGER
    RATIO -->|no| WINDOW

    MISS --> CLEANUP{"窗口大小 > W?"}
    CLEANUP -->|yes| EVICT["移除最旧 token<br/>+ 清理过期 n-gram"]
    CLEANUP -->|no| WINDOW

    TRIGGER --> CANCEL["调用 cancel_generation()"]
    CANCEL --> COLLECT["收集已输出 token<br/>作为错误现场"]
    COLLECT --> RECOVER["触发恢复管理器"]

    style TRIGGER fill:#ffebee
    style RECOVER fill:#fff3e0
```

**选择**: 经典 n-gram + 滑动窗口检测。连续命中阈值和窗口重复占比双条件，任一触发即判定循环。

**数据结构**:
- `VecDeque<u32>` 滑动窗口（token ID）
- `HashSet<Vec<u32>>` n-gram 集合（动态长度 `min_n..=max_n`）
- `consecutive_hits: u32` 连续命中计数器

**权衡**: HashSet 比 Bloom filter 精确无假阳性，但内存占用更高。窗口大小 W=50 时内存可忽略。

### Decision 2: 中间件集成位置

```mermaid
flowchart LR
    LOOP["Agent Loop"] -->|"请求生成"| MODEL["Model Provider<br/>(adk-model)"]
    MODEL -->|"流式 token"| DETECTOR["RepeatDetector<br/>中间件"]
    DETECTOR -->|"正常 token"| LOOP
    DETECTOR -->|"检测到循环"| CANCEL["cancel_generation()"]
    CANCEL --> RECOVERY["RecoveryManager"]

    subgraph "透明中间件层"
        DETECTOR
    end

    style DETECTOR fill:#fff3e0
```

**选择**: RepeatDetector 作为 `Stream` 装饰器，包装模型输出流。对上层（agent loop）完全透明——正常的 token 照常传递，循环时流提前终止。

**实现方式**: 实现 `Stream<Item = Token>` trait 的装饰器，内部维护检测状态。

### Decision 3: 恢复策略链

```mermaid
flowchart TD
    DETECT["检测到循环"] --> STRATEGY{"recovery.strategy?"}

    STRATEGY -->|"action 1:<br/>alter_prompt"| ALTER["修改 prompt<br/>加入反重复指令"]
    ALTER --> RETRY1["重试同模型"]
    RETRY1 --> CHECK1{"再次循环?"}
    CHECK1 -->|no| SUCCESS["恢复成功"]
    CHECK1 -->|yes| NEXT1{"还有下一个 action?"}

    NEXT1 -->|"action 2:<br/>switch_model"| SWITCH["切换到另一 Provider<br/>（OpenAI ↔ Anthropic）"]
    SWITCH --> RETRY2["重试新模型"]
    RETRY2 --> CHECK2{"再次循环?"}
    CHECK2 -->|no| SUCCESS
    CHECK2 -->|yes| NEXT2{"还有下一个 action?"}

    NEXT2 -->|"action 3:<br/>adjust_params"| ADJUST["提高惩罚参数<br/>repetition_penalty"]
    ADJUST --> RETRY3["重试同模型"]
    RETRY3 --> CHECK3{"再次循环?"}
    CHECK3 -->|no| SUCCESS
    CHECK3 -->|yes| NEXT3{"还有下一个 action?"}

    NEXT3 -->|"action 4:<br/>delegate_to_planner"| DELEGATE["报告规划器<br/>请求重新规划"]
    DELEGATE --> DONE2["恢复流程结束"]

    SUCCESS --> DONE["继续正常执行"]

    style SUCCESS fill:#e8f5e9
    style DELEGATE fill:#ffebee
```

**选择**: sequential 恢复策略——按配置顺序依次尝试 4 种 action，直到成功或全部失败。`max_attempts` 限制总尝试次数。

**权衡**: sequential 比 parallel 简单且可预测。parallel 可能更快但浪费 API 调用（同时请求多个模型）。

### Decision 4: 配置映射

```yaml
repeat_detection:
  enabled: true
  min_n: 3
  max_n: 10
  window_size: 50
  consecutive_hit_threshold: 5
  window_repeat_ratio: 0.8
  early_stop_tokens: 100

  recovery:
    strategy: "sequential"
    max_attempts: 3
    actions:
      - type: "alter_prompt"
        prepend: "WARNING: Avoid repetition."
      - type: "switch_model"
        model_id: null          # null = 切换到另一 provider（OpenAI ↔ Anthropic）
      - type: "adjust_params"
        params:
          repetition_penalty: 1.4
      - type: "delegate_to_planner"
```

**选择**: 直接映射 PRD §10.5 配置结构。`early_stop_tokens` 作为性能优化——输出超过此长度未检测到重复则停止监控。

## Risks / Trade-offs

| 风险 | 等级 | 缓解 |
|------|------|------|
| n-gram HashSet 内存增长（长输出） | 低 | early_stop_tokens 限制监控长度；LRU 清理过期 n-gram |
| cancel_generation() 抽象不完整（不同后端行为不一致） | 中 | 依赖 adk-model 统一抽象；fallback 到关闭 HTTP 流 |
| 误判（代码中合法重复被中断） | 中 | consecutive_hit_threshold 可调高；代码模式可通过配置排除 |
| 恢复策略全部失败 | 低 | 最终兜底 delegate_to_planner 让规划器重新分解任务 |

### 待确认问题

- 无
