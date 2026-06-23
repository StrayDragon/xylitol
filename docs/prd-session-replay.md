# PRD: Session Replay & Time Travel — 会话回放与时间旅行调试

> 状态: Idea v1
> 优先级: P2 (探索性)
> 预计工作量: 2-3 周
> 依赖: TUI PRD 完成，Session 持久化机制 (infra-session)

## 1. 概述

### 1.1 愿景

为 xylitol 实现**会话回放和时间旅行调试**功能，让用户能够：

1. **回放历史会话** — 观看 agent 的完整思考和执行过程
2. **时间旅行** — 在会话历史中任意跳转，查看任意时刻的状态
3. **分支实验** — 从历史某点分叉，尝试不同的提问方式
4. **教学录制** — 将优秀的 agent 交互录制为教程

### 1.2 核心价值

| 场景 | 当前痛点 | 解决方案 |
|------|----------|----------|
| 调试 agent 行为 | 不知道 agent 为什么做出某个决定 | 回放完整思考过程 |
| 学习最佳实践 | 看不到别人怎么用 agent | 观看录制的优秀会话 |
| 复现问题 | 无法重现 bug | 回放到问题发生的时刻 |
| 尝试不同路径 | 只能从头开始 | 从历史某点分叉继续 |
| 团队协作 | 无法分享 agent 交互过程 | 导出和分享会话录制 |

### 1.3 灵感来源

- **Redux DevTools** 的时间旅行调试
- **Git** 的分支和历史管理
- **Loom** 的屏幕录制
- **Jupyter Notebook** 的单元格执行历史

## 2. 功能设计

### 2.1 核心概念

#### 时间线 (Timeline)
```
会话时间线:
  T0 ──T1──T2──T3──T4──T5──T6──T7──T8──T9
  │    │   │   │   │   │   │   │   │   │
  用户  助手 工具  助手 用户  助手 工具  工具  助手 用户
  消息  消息 调用  消息 消息  消息 调用  调用  消息 消息
```

#### 检查点 (Checkpoint)
```
T0 (Start) ─── T3 (Checkpoint) ─── T6 (Checkpoint) ─── T9 (End)
                  │                      │
                  └── Branch A           └── Branch B
                      (尝试不同路径)          (继续原路径)
```

#### 状态快照 (State Snapshot)
```rust
struct StateSnapshot {
    timestamp: Instant,
    index: usize,
    // Agent 状态
    transcript: Transcript,
    context_window: Vec<Message>,
    // 外部状态
    file_changes: HashMap<PathBuf, FileDiff>,
    // 元数据
    metadata: SnapshotMetadata,
}
```

### 2.2 回放模式

#### 2.2.1 自动回放 (Auto Replay)
```
[▶ Play] [⏸ Pause] [⏹ Stop] [⏩ 2x] [⏩ 4x]

速度: ████████████░░░░░░░░ 60%
时间: 00:02:34 / 00:05:12

当前步骤:
  Agent: 我来分析这个函数的性能问题...
  [正在执行 bash 命令: time python script.py]
```

**控制**:
- `Space` — 暂停/继续
- `→` — 下一步
- `←` — 上一步
- `Ctrl+→` — 跳到下一个检查点
- `Ctrl+←` — 跳到上一个检查点

#### 2.2.2 步进模式 (Step Mode)
```
Step 3/15: Tool Call - bash

┌─────────────────────────────────────────────┐
│ 命令: ls -la /tmp                           │
│ 输出:                                       │
│   total 12                                  │
│   drwxrwxrwt 2 user user 4096 Jun 23 10:00 .│
│   drwxr-xr-x 3 root root 4096 Jun 23 09:00 ..│
│                                             │
│ [Previous] [Next] [Skip to End] [Branch]    │
└─────────────────────────────────────────────┘
```

#### 2.2.3 概览模式 (Overview Mode)
```
┌─────────────────────────────────────────────┐
│ 会话概览: 代码重构任务                        │
│ 时长: 5分12秒 | 消息: 24 | 工具调用: 12      │
├─────────────────────────────────────────────┤
│ 时间线:                                      │
│ ──●────●────●────●────●────●────●────●──    │
│   U    A    T    A    U    A    T    A       │
│                                             │
│ 关键事件:                                    │
│ [T2] bash: 发现性能瓶颈                      │
│ [T5] edit: 重构循环逻辑                      │
│ [T7] bash: 测试通过 ✓                        │
└─────────────────────────────────────────────┘
```

### 2.3 时间旅行 (Time Travel)

#### 2.3.1 跳转到历史点
```
时间旅行模式:

  T0 ──T1──T2──T3──T4──T5──T6──T7──T8──T9
                  ▲
                  │
            当前位置: T3 (Agent 分析代码结构)

[Go to T3] [View State] [Branch Here] [Return to Present]
```

#### 2.3.2 查看历史状态
```
时间点 T3 的状态:

┌─────────────────────────────────────────────┐
│ 对话历史:                                    │
│   User: 重构 auth 模块                       │
│   Agent: 我来分析代码结构...                  │
│   Agent: 发现 3 个主要问题:                   │
│     1. 函数过长 (200+ 行)                    │
│     2. 职责不明确                            │
│     3. 缺少错误处理                          │
├─────────────────────────────────────────────┤
│ 文件状态:                                    │
│   auth.py (modified)                        │
│   tests/test_auth.py (unchanged)            │
├─────────────────────────────────────────────┤
│ [Back to Present] [Branch from Here]        │
└─────────────────────────────────────────────┘
```

#### 2.3.3 分支实验 (Branching)
```
从 T3 创建分支:

原始时间线: T0 → T1 → T2 → T3 → T4 → T5
                              │
                              └── Branch: "try different approach"
                                  T3' → T4' → T5' → T6'

分支描述: 尝试使用不同的重构策略
```

**分支操作**:
- `b` — 从当前位置创建分支
- 输入分支描述
- 在新分支中继续对话
- 可以随时切换回主分支
- 可以合并分支结果

### 2.4 录制与导出

#### 2.4.1 自动录制
```rust
struct SessionRecording {
    session_id: String,
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
    events: Vec<RecordedEvent>,
    checkpoints: Vec<Checkpoint>,
    metadata: RecordingMetadata,
}

struct RecordedEvent {
    index: usize,
    timestamp: Instant,
    event_type: EventType,
    data: EventData,
    // 用于快速预览
    summary: String,
}
```

#### 2.4.2 导出格式

**Markdown 格式** (用于文档):
```markdown
# Session: 代码重构任务
Date: 2026-06-23
Duration: 5m 12s

## 对话

### User
重构 auth 模块

### Agent
我来分析代码结构...

发现 3 个主要问题:
1. 函数过长 (200+ 行)
2. 职责不明确
3. 缺少错误处理

[工具调用] bash: `ls -la auth/`
[输出] ...

## 结论
成功将 auth 模块重构为 3 个独立文件。
```

**JSON 格式** (用于程序处理):
```json
{
  "session_id": "abc123",
  "start_time": "2026-06-23T10:00:00Z",
  "events": [
    {
      "index": 0,
      "type": "user_message",
      "content": "重构 auth 模块",
      "timestamp": 0
    },
    ...
  ],
  "checkpoints": [
    {
      "index": 3,
      "label": "问题分析完成",
      "state_snapshot": {...}
    }
  ]
}
```

**视频格式** (用于分享):
- 使用 asciinema 录制终端输出
- 支持导出为 GIF/MP4
- 可添加字幕和注释

## 3. 技术架构

### 3.1 录制系统

```rust
struct SessionRecorder {
    // 事件流
    event_buffer: VecDeque<RecordedEvent>,
    // 检查点管理
    checkpoints: Vec<Checkpoint>,
    // 状态快照
    snapshots: BTreeMap<usize, StateSnapshot>,
    // 存储
    storage: RecordingStorage,
}

impl SessionRecorder {
    fn record_event(&mut self, event: AgentEvent) {
        let recorded = RecordedEvent::from_agent_event(event);
        self.event_buffer.push_back(recorded);

        // 每 N 个事件自动创建检查点
        if self.should_checkpoint() {
            self.create_checkpoint();
        }
    }

    fn create_checkpoint(&mut self) -> Checkpoint {
        let snapshot = self.capture_state();
        let checkpoint = Checkpoint {
            index: self.current_index(),
            snapshot,
            label: self.auto_label(),
        };
        self.checkpoints.push(checkpoint.clone());
        checkpoint
    }

    fn capture_state(&self) -> StateSnapshot {
        StateSnapshot {
            timestamp: Instant::now(),
            index: self.current_index(),
            transcript: self.transcript.clone(),
            file_changes: self.collect_file_changes(),
            metadata: self.collect_metadata(),
        }
    }
}
```

### 3.2 回放引擎

```rust
struct ReplayEngine {
    recording: SessionRecording,
    current_index: usize,
    playback_speed: f64,
    state: PlaybackState,
}

impl ReplayEngine {
    fn play(&mut self) {
        self.state = PlaybackState::Playing;
        // 使用 timer 按时间戳回放事件
    }

    fn pause(&mut self) {
        self.state = PlaybackState::Paused;
    }

    fn step_forward(&mut self) -> Option<&RecordedEvent> {
        if self.current_index < self.recording.events.len() {
            let event = &self.recording.events[self.current_index];
            self.current_index += 1;
            Some(event)
        } else {
            None
        }
    }

    fn step_backward(&mut self) -> Option<&RecordedEvent> {
        if self.current_index > 0 {
            self.current_index -= 1;
            Some(&self.recording.events[self.current_index])
        } else {
            None
        }
    }

    fn jump_to(&mut self, index: usize) {
        self.current_index = index.min(self.recording.events.len());
    }

    fn jump_to_checkpoint(&mut self, checkpoint_index: usize) {
        if let Some(checkpoint) = self.recording.checkpoints.get(checkpoint_index) {
            self.jump_to(checkpoint.index);
        }
    }
}
```

### 3.3 分支管理

```rust
struct BranchManager {
    // 主时间线
    main_timeline: Timeline,
    // 分支
    branches: HashMap<String, Branch>,
    // 当前活跃分支
    active_branch: Option<String>,
}

struct Branch {
    id: String,
    name: String,
    description: String,
    fork_point: usize,  // 从主时间线的哪个点分叉
    timeline: Timeline,
    created_at: DateTime<Utc>,
}

impl BranchManager {
    fn create_branch(&mut self, fork_point: usize, name: &str) -> String {
        let branch_id = uuid::Uuid::new_v4().to_string();
        let branch = Branch {
            id: branch_id.clone(),
            name: name.to_string(),
            description: String::new(),
            fork_point,
            timeline: self.main_timeline.fork_at(fork_point),
            created_at: Utc::now(),
        };
        self.branches.insert(branch_id.clone(), branch);
        branch_id
    }

    fn switch_branch(&mut self, branch_id: &str) {
        // 保存当前状态
        // 加载目标分支状态
        self.active_branch = Some(branch_id.to_string());
    }

    fn merge_branch(&mut self, branch_id: &str) -> Result<()> {
        // 将分支的修改合并到主时间线
        todo!()
    }
}
```

## 4. TUI 集成

### 4.1 回放界面

```
┌─────────────────────────────────────────────┐
│ Session Replay: 代码重构任务                  │
├─────────────────────────────────────────────┤
│                                             │
│  [▶] [⏸] [⏹] [2x] [4x]  ████░░░░ 60%     │
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │ 对话内容...                          │   │
│  │                                      │   │
│  │ User: 重构 auth 模块                 │   │
│  │ Agent: 我来分析...                   │   │
│  │ [工具调用] bash: ls -la              │   │
│  │                                      │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  检查点: ●────●────●────●────●              │
│          T0   T3   T6   T9   T12           │
│                                             │
│  [Previous] [Next] [Jump] [Branch] [Export] │
└─────────────────────────────────────────────┘
```

### 4.2 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Space` | 暂停/继续 |
| `→` | 下一步 |
| `←` | 上一步 |
| `Ctrl+→` | 下一个检查点 |
| `Ctrl+←` | 上一个检查点 |
| `g` | 跳转到指定位置 |
| `b` | 创建分支 |
| `e` | 导出 |
| `q` | 退出回放 |
| `1-9` | 调整播放速度 |

### 4.3 命令行接口

```bash
# 回放最近的会话
xylitol replay

# 回放指定会话
xylitol replay --session abc123

# 从检查点开始回放
xylitol replay --session abc123 --from-checkpoint 2

# 导出会话
xylitol export --session abc123 --format markdown --output session.md

# 列出所有会话
xylitol sessions list

# 查看会话详情
xylitol sessions info abc123
```

## 5. 存储设计

### 5.1 录制文件结构

```
~/.xylitol/recordings/
├── abc123/
│   ├── metadata.json      # 会话元数据
│   ├── events.jsonl       # 事件流 (JSON Lines)
│   ├── checkpoints.json   # 检查点索引
│   ├── snapshots/         # 状态快照
│   │   ├── 0.json
│   │   ├── 3.json
│   │   └── 6.json
│   └── branches/          # 分支
│       ├── def456.json
│       └── ghi789.json
└── index.json             # 全局索引
```

### 5.2 增量存储

```rust
struct IncrementalStorage {
    // 基础快照
    base_snapshot: StateSnapshot,
    // 增量更新
    deltas: Vec<StateDelta>,
}

struct StateDelta {
    index: usize,
    changes: Vec<Change>,
}

enum Change {
    MessageAdded { index: usize, message: Message },
    MessageUpdated { index: usize, field: String, value: Value },
    ToolCallAdded { id: String, call: ToolCall },
    ToolCallUpdated { id: String, status: ToolStatus },
    FileChanged { path: PathBuf, diff: FileDiff },
}
```

## 6. 实施阶段

### Phase 1: 基础录制 (1 周)
- [ ] 实现 SessionRecorder
- [ ] 事件流持久化 (JSONL)
- [ ] 基础元数据记录

### Phase 2: 回放引擎 (1 周)
- [ ] 实现 ReplayEngine
- [ ] 播放控制 (play/pause/step)
- [ ] TUI 回放界面

### Phase 3: 检查点与快照 (3 天)
- [ ] 自动检查点创建
- [ ] 状态快照机制
- [ ] 跳转到检查点

### Phase 4: 分支系统 (3 天)
- [ ] 分支创建和切换
- [ ] 分支可视化
- [ ] 基础合并功能

### Phase 5: 导出与分享 (3 天)
- [ ] Markdown 导出
- [ ] JSON 导出
- [ ] 会话索引和搜索

### Phase 6: 高级功能 (持续)
- [ ] 视频录制 (asciinema)
- [ ] 协作分享
- [ ] 教程录制模式

## 7. 使用场景

### 场景 1: 调试 agent 行为
```
问题: agent 为什么选择删除这个文件？

步骤:
1. xylitol replay --session problematic-session
2. 跳转到删除文件的时刻
3. 查看 agent 的思考过程
4. 发现是因为误解了用户意图
5. 创建分支，尝试更明确的提示
```

### 场景 2: 创建教程
```
目标: 创建一个"如何使用 xylitol 重构代码"的教程

步骤:
1. 正常使用 xylitol 完成重构任务
2. xylitol export --session abc123 --format markdown
3. 编辑导出的 Markdown，添加注释
4. 分享给团队成员
```

### 场景 3: 尝试不同策略
```
任务: 优化这个函数的性能

步骤:
1. 使用 xylitol 完成第一种优化方案
2. 回放到分析阶段 (T3)
3. 创建分支 "strategy-2"
4. 在分支中尝试不同的优化策略
5. 比较两个分支的结果
6. 选择更好的方案
```

## 8. 竞品分析

| 特性 | xylitol Replay | Redux DevTools | Git History | Loom |
|------|----------------|----------------|-------------|------|
| 时间旅行 | ✓ | ✓ | ✗ | ✗ |
| 分支实验 | ✓ | ✗ | ✓ | ✗ |
| 自动录制 | ✓ | ✓ | ✗ | ✓ |
| 导出分享 | ✓ | ✗ | ✓ | ✓ |
| AI 专用 | ✓ | ✗ | ✗ | ✗ |
| 终端集成 | ✓ | ✗ | ✓ | ✗ |

## 9. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 存储空间 | 大量会话占用磁盘 | 增量存储 + 压缩 + 自动清理 |
| 性能影响 | 录制可能影响 agent 性能 | 异步录制 + 批量写入 |
| 状态一致性 | 时间旅行可能产生不一致状态 | 只读回放 + 明确的分支边界 |
| 隐私问题 | 录制可能包含敏感信息 | 加密存储 + 访问控制 |

## 10. 成功指标

| 指标 | 目标 | 衡量方式 |
|------|------|----------|
| 录制开销 | <5% 性能影响 | 性能测试 |
| 回放流畅度 | 60fps | 用户体验测试 |
| 存储效率 | 压缩率 >70% | 存储测试 |
| 用户采用率 | >30% 用户使用 | 使用统计 |

## 11. 开放问题

1. **录制粒度**: 应该录制每个 token 还是每个消息？
2. **分支合并**: 如何处理分支间的冲突？
3. **协作录制**: 如何支持多人协作的会话录制？
4. **隐私保护**: 如何在录制中保护敏感信息？

## 12. 参考资料

- [Redux DevTools](https://github.com/reduxjs/redux-devtools) - 时间旅行调试
- [asciinema](https://asciinema.org/) - 终录制
- [Git](https://git-scm.com/) - 版本控制和分支
- [Loom](https://www.loom.com/) - 屏幕录制和分享
