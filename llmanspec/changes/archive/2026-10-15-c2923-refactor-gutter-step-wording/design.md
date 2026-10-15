# c2923 设计：词汇映射与改名机制

## 决策

1. **映射表（唯一真源，全库机械替换）**

| 旧 | 新 | 语义不变式 |
|---|---|---|
| 跳顶 | 跳转顶部 | 点击后应用视口滚动归零 |
| 跳底 | 跳转底部 | 点击后滚到底并重续跟随 |
| 跳顶/跳底记号 | 跳转顶部/底部按钮字符 | 可点击字符常量 `GUTTER_TOP='^'` / `GUTTER_BOTTOM='v'` |
| 右缘（占列语义） | 最右列 | gutter/侧栏占据的最右 1 列 |
| 右缘（边界语义） | 右边界 / 屏幕右侧 | 贴内容右边界；屏幕右侧浮层 |

2. **步骤短语改名范围**：仅 when 短语键与 then 产品组合短语；given/绑定名/场景句柄/代码标识符（`GUTTER_TOP_HIT_ID` 等）不动——binding 以 `(path,name)` 寻址，不随措辞变。

3. **新旧同现策略**：变更 proposal 内保留旧词作对照（映射表即文档）；specs/tests/docs 活跃面清零，archive 历史不追溯。

## 影响面

- feature 3：package-tui-scrollbar-gutter（7 处）、app-tui-transcript r2893（3）、app-tui-fixed-zone 侧栏/弹出框（5）
- steps 短语键 2 文件 6 键；源码 doc/注释 5 文件；词汇表+术语表各 1 行
- pageindex 缓存经 `index rebuild` 刷新
