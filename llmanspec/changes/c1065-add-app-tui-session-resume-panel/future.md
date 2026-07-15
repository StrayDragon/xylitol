# Future — c1065

## Deferred Items

| 项 | 原因 | 触发重开 |
|---|---|---|
| 扫描中 live `loaded/total`（逐文件刷新标题） | 用户确认可后做；需增量 list API + 每步 render | 大目录加载体感卡住；或明确要求对齐 pi Loading N/M 动画 |
| `/session-resume <id>` 直参 | MVP 对齐 pi 无参开板 | 脚本化/自动化切换需求 |
| 多 project 全局 sessions 根 | 视 xylitol 存储布局演进 | 引入 pi 式按 cwd 分桶目录 |

## Branch Options

- 包内通用 `SessionPicker` 组件 vs 产品私有面板：本 change 产品私有；若第二表面复用再抽 `package-tui-*`。
