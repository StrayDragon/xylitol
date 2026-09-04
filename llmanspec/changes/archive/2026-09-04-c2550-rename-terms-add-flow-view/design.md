# Design: c2550-rename-terms-add-flow-view

## 词映射总表（SSOT 顺序）

| 旧 | 新 | 英文锚 / 标识符 | 说明 |
|---|---|---|---|
| 壳层通告 | **通知条** | toast notice · `ToastNotice` · `push_toast_notice` | 底部输入区上方（status/spinner 之上）瞬时通知；warning 色 + `Error: ` 前缀 + TTL 自动清除 |
| 尾随 | **尾插** | tail-append | append 到 entries 末；「顶插」不变 |
| chrome | **固定区** | fixed zone | 状态条/页脚/通知条/槽等非滚动固定框架区统称 |

弃用表（词表内）新增：壳层通告→通知条、尾随→尾插、chrome（泛固定区旧词）→固定区。归档 change 不改写（词表维护规则允许史实旧词）。

## 代码标识符映射（产品面 src/）

| 旧 | 新 | 语义 |
|---|---|---|
| `push_chrome_toast` | `push_toast_notice` | 通知条入槽（与 `push_scroll_notice` 成对） |
| `chrome_toast`（字段/变量） | `toast_notice` | 布局根通知条槽 |
| `chrome_toast_body`（harness） | `toast_notice_body` | 测试助手 |
| `render_chrome_toast_slot` / `expire_chrome_toast_now` / `clear_chrome_toast_if_expired` / `clear_chrome_toast` | `render_toast_notice_slot` / `expire_toast_notice_now` / `clear_toast_notice_if_expired` / `clear_toast_notice` | 渲染与生命周期 |
| `ChromeOp` / `PreviewInject::Chrome` | `FixedZoneOp` / `PreviewInject::FixedZone` | debug fixture 注入枚举（本地，不经 wire） |
| `apply_chrome_op` | `apply_fixed_zone_op` | fixture 应用 |
| `SlotChromeRows` / `chrome_footprint` / `apply_chrome_footprint` / `chrome_footprint_apply.rs` / `layout/chrome_footprint.rs` | `SlotFixedZoneRows` / `fixed_zone_footprint` / `apply_fixed_zone_footprint` / `fixed_zone_footprint_apply.rs` / `layout/fixed_zone_footprint.rs` | 固定区行预算 SSOT |
| `sync_runtime_chrome` | `sync_fixed_zone` | driver 运行时状态 → 固定区（footer/mcp cue） |
| `set_active_chrome` | `set_active_fixed_zone` | 布局根当前固定区内容 |
| `refresh_chrome_caches` | `refresh_fixed_zone_caches` | MCP/skills 固定区资源缓存 |
| `effects/slash/chrome.rs` | `effects/slash/misc.rs`（原文档注释自称 "Chrome / misc"，按实际内容定名） | slash 杂项臂 |
| `chrome_hint` / `reserved_lower_chrome` / 其余散置 `chrome` 词形 | 按所在语义改 `fixed_zone*` / `toast_notice*`；中文注释「chrome」改「固定区」 | 逐处辨义 |
| designing 模块 id `chrome-toast` | `toast-notice` | 目录/路由/draft id/states id/引用链接 |

**不改**：`packages/xylitol-tui` 内 fence chrome / demo chrome 等泛指用法（引擎 fork 上游词汇；其 AGENTS 已声明词表只约束产品面文档与 host）；wire 协议形状；`ScrollNotice`/`push_scroll_notice`（本就正确）。

## designing playground

### 交接卡（handoff，不改名）遮盖修复

成因：`.handoff` 为 `position: sticky; top: 0` + 不透明背景 + 16 条 bullet 的大块，滚动时把 `#detail` 盖住。

方案：改为单行 sticky 条（`复制路径` 按钮 + endpoint 截断 + 右侧「展开 ▾」）；元数据与路径清单移入展开面板（普通流区块，滚动即走，非 sticky）；交接卡区与 `#detail` 以背景/边框隔离。`id="copy-handoff"` 保留（过 check_tui_designing 闸）。

### flow.yaml 时态平铺图（仅 3 个 tui-lab 模块先落地）

```yaml
initial: busy
nodes:
  - { state: busy, label: 进行中 }      # state → states/<state>.yaml 快照
edges:
  - { from: busy, to: armed, trigger: "Esc", kind: key }    # kind: key | timeout | auto
  - { from: armed, to: busy, trigger: "5s", kind: timeout }
demo: [busy, armed, stopped]            # 播放路径（动态设计）
```

- 渲染器 `app/src/flow-view.ts`：SVG 贝塞尔边 + 箭头 + 触发标签（key=kbd 样式、timeout=虚线+秒数）；节点卡 = label + 缩略 cell-grid 预览（点击跳该态路由）；时态自左向右自动分层（拓扑深度，initial 最左）；播放 = 沿 demo 路径高亮当前节点/活动边（dash 动画，timeout 边倒数角标）
- 删除键盘黑盒 sim：3× `sim.ts` + `app/src/sim.ts` + `main.ts` live 机制 + live CSS；chip「交互原型」→「时态图」（state 段 `__flow__`，保持 pathname 路由）
- 补缺失时态快照（如 interrupt-arm 增 busy/stopped）
- 手写轻量实现不加框架：该规模（≤5 节点/模块）React+xyflow 不减总代码反而引入第二套渲染范式；渲染器按可复用模块写，推广成本低

### alignment 键与 UI 标签

draft.yaml `alignment.chrome` → `alignment.fixed`（约 40 个 draft 机械替换 + `types.ts`/`main.ts`）；UI dt 显示中文标签映射：fixed→固定区、item→对话条目、todo-bar→待办栏。

## 权衡

- **capability 名一并改**（app-tui-chrome → app-tui-fixed-zone）：历史名已无解释成本收益，一次改全符合 pre-0.0.1 无兼容债纪律；场景 id 同步改。
- **specs 纯措辞变更**（MUST 语义不变）与新增 adp req 同 change 落地：避免 designing 内新旧词混用的中间态。
- 归档区不改写旧词：词表维护规则明确归档可保留史实旧词。
