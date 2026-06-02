# 源码引用索引

| 文件路径 | 行号 | 章节 | 说明 |
|---------|------|------|------|
| `packages/opencode/src/cli/cmd/tui/app.tsx` | L1-L28 | 1,2,3 | 主入口、imports、Provider 树 |
| `packages/opencode/src/cli/cmd/tui/app.tsx` | L82-L124 | 2 | App 级命令注册列表 |
| `packages/opencode/src/cli/cmd/tui/app.tsx` | L126-L147 | 2,3 | Renderer 配置 |
| `packages/opencode/src/cli/cmd/tui/app.tsx` | L166-L265 | 1,2,9 | TUI 初始化函数 |
| `packages/opencode/src/cli/cmd/tui/app.tsx` | L267-L981 | 4,6 | App 组件、路由、命令注册、事件监听 |
| `packages/opencode/src/cli/cmd/tui/app.tsx` | L352-L375 | 8 | 终端标题同步 |
| `packages/opencode/src/cli/cmd/tui/app.tsx` | L744-L749 | 8 | SIGTSTP/SIGCONT 处理 |
| `packages/opencode/src/cli/cmd/tui/app.tsx` | L942-L981 | 4 | 根组件 JSX 结构 |
| `packages/opencode/src/cli/cmd/tui/keymap.tsx` | L41-L88 | 2,8 | Mode Stack 实现 |
| `packages/opencode/src/cli/cmd/tui/keymap.tsx` | L100-L122 | 2 | 键别名展开 |
| `packages/opencode/src/cli/cmd/tui/keymap.tsx` | L124-L161 | 2 | Input 命令列表 |
| `packages/opencode/src/cli/cmd/tui/keymap.tsx` | L196-L227 | 2 | registerOpencodeKeymap |
| `packages/opencode/src/cli/cmd/tui/event.ts` | L1-L54 | 2 | TUI 内部事件定义 |
| `packages/opencode/src/cli/cmd/tui/context/sync.tsx` | L37-L108 | 5 | SyncStore 初始结构 |
| `packages/opencode/src/cli/cmd/tui/context/sync.tsx` | L133-L373 | 5,6 | 事件订阅处理器 |
| `packages/opencode/src/cli/cmd/tui/context/sync.tsx` | L327-L343 | 3,5 | Delta 流式追加 |
| `packages/opencode/src/cli/cmd/tui/context/sync.tsx` | L378-L479 | 5,8 | Bootstrap 分阶段加载 |
| `packages/opencode/src/cli/cmd/tui/context/theme.tsx` | L89-L123 | 7 | 内置主题注册 |
| `packages/opencode/src/cli/cmd/tui/context/theme.tsx` | L199-L257 | 7 | 主题解析（resolveTheme） |
| `packages/opencode/src/cli/cmd/tui/context/theme.tsx` | L304-L486 | 7 | ThemeProvider 实现 |
| `packages/opencode/src/cli/cmd/tui/context/theme.tsx` | L522-L631 | 7 | System 主题生成 |
| `packages/opencode/src/cli/cmd/tui/context/theme.tsx` | L633-L716 | 7,8 | 灰度/柔色自适应 |
| `packages/opencode/src/cli/cmd/tui/context/helper.tsx` | L1-L25 | 4,8 | createSimpleContext 工厂 |
| `packages/opencode/src/cli/cmd/tui/context/route.tsx` | L1-L53 | 5 | 路由状态管理 |
| `packages/opencode/src/cli/cmd/tui/context/sdk.tsx` | L1-L72 | 5,8 | SDK + SSE 事件批处理 |
| `packages/opencode/src/cli/cmd/tui/context/event.ts` | L1-L39 | 5 | 事件订阅 hook |
| `packages/opencode/src/cli/cmd/tui/context/local.tsx` | L1-L100 | 5 | 本地 UI 状态 |
| `packages/opencode/src/cli/cmd/tui/component/spinner.tsx` | L1-L24 | 6,7 | Spinner + 动画降级 |
| `packages/opencode/src/cli/cmd/tui/component/prompt/index.tsx` | L134-L1805 | 2,4,6,8 | Prompt 完整实现 |
| `packages/opencode/src/cli/cmd/tui/component/prompt/index.tsx` | L466-L491 | 6 | 三次 Esc 中断 |
| `packages/opencode/src/cli/cmd/tui/component/prompt/index.tsx` | L993-L1008 | 8 | Submit 防重入 |
| `packages/opencode/src/cli/cmd/tui/component/prompt/index.tsx` | L1305-L1311 | 8 | 粘贴长文本折叠 |
| `packages/opencode/src/cli/cmd/tui/component/prompt/index.tsx` | L1510-L1511 | 8 | IME 双延迟 |
| `packages/opencode/src/cli/cmd/tui/ui/dialog.tsx` | L1-L219 | 4 | Dialog 栈系统 |
| `packages/opencode/src/cli/cmd/tui/routes/session/index.tsx` | L1-L200 | 4 | Session 路由 |
| `packages/opencode/src/cli/cmd/tui/win32.ts` | L1-L131 | 2,8 | Windows 平台处理 |
| `packages/opencode/src/cli/cmd/tui/attention.ts` | L1-L262 | 6 | 通知/声音/焦点 |

> **路径约定**：所有路径相对于 `packages/opencode/src/` 前缀，完整路径形如 `packages/opencode/src/cli/cmd/tui/app.tsx`。
