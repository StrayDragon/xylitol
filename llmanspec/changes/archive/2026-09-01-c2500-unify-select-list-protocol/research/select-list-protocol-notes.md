# SelectList 协议收敛 调研补充（c2500）

> 2026-08-23 代码事实核对 + 引入设计。结论先行：**包层已有 SelectList 组件**，
> 本票不是新建，而是「协议收敛」——补齐缺口能力并让全部列表槽走同一契约。

## 现状核对（代码事实）

**包层已有抽象**（`packages/xylitol-tui/src/components/select_list.rs`）：

- `SelectItem { value, label, description }`（:27-50）
- `SelectList`：`new` / `set_filter` / `set_selected_index` / `get_selected_item`（:87-135）
- 配套 `SelectListTheme` / `SelectListLayoutOptions` / `SelectListTruncatePrimaryContext`
- 经 `lib.rs:58-59` re-export，属包 API 边界内

**应用侧使用分布**（各自组装、行为有差异）：

| 槽 | 文件 | 用法 |
|---|---|---|
| import 确认 | `layout/slots/import.rs` | SelectList |
| models | `layout/slots/models.rs` + `models_picker.rs` | `ModelPickerRow::to_select_item(focused, width_budget)` |
| themes | `layout/slots/themes.rs` | SelectList |
| mcp | `layout/slots/mcp.rs` | SelectList |
| session resume | `session_resume/panel.rs`（36 行起） | **自绘 panel**：`load_entries` / 分批加载 `set_loading(loaded,total)` / `apply_rename` / `search.rs` 防抖 |

## 与外部参照的差距（手法摘录）

成熟 agent TUI 的通用列表对话框在同等组件上多出三个能力：

1. **分组头**（Favorites / Recent / 按 provider…）：过滤时分组头固定参与匹配；
2. **底部 action 条**（次要操作做成 footer 按钮 + Tab 焦点环，避免二级弹窗嵌套）；
3. **details 多行说明区**（选中项展开多行描述，而非单行 description 截断）。

另有两点工程纪律：过滤为纯函数可直测；选中项身份稳定（重过滤不跳行）。

## 引入设计（规格草案）

- 包层 `SelectItem` 增加**可选 group 键**与 `details: Vec<String>`；`SelectList`
  渲染分组头行、维护「组头不可选中、过滤命中组则组头保留」规则。
- 新增 action 条协议：槽声明 `(chord, label)` 列表 → SelectList 底部渲染 +
  `Tab` 在列表/action 条间移动焦点；动作回调归应用槽。
- resume 自绘 panel 迁移到该协议：分批加载与 rename/delete 作为 action 条动作；
  「预览按终端比例软顶」「Ctrl+U 展开 id」等既有决议保持不变（PI 对齐项）。
- 过滤逻辑抽纯函数进包测试层。

## 决策点

1. resume 是否强制迁移（其搜索/分批最复杂，迁移收益最大也最险）；
2. action 条键位冲突策略（与全局键的优先级）；
3. 分组头是否允许折叠（首版建议不允许）。

## 影响面

- 包层组件 + 单测；五个槽接线改造；designing 各对应模块 states 更新；
- 不改 `app::product_commands` SSOT（命令目录仍归 app）。
