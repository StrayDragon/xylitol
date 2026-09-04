# Design：c2540 簇头文件分句统一计数式

## D1 计数式 vs basename（用户拍板）

basename 是 att24 的既有设计（单路径时给「哪个文件」的一眼信息），但人工实测
反馈：簇头一闪而过，basename 读不到价值、更像内容回显；具体文件展开簇内子块
即可获得。c2510 类目计数后缀落地后，`Explored Cargo.toml · 2 reads` 的
「名词 + 两类数字」混排进一步放大违和。收敛为：文件分句恒为计数式
（`1 file` / `N files`），Edited / Explored 同规则，进行时词形同构。

## D2 rules_edit_acked 的使用理由

att24 是既有 `@human` 场景，文件分句句改必须改其文本——本票是该条款的直接
修订者（非顺带触碰），frontmatter 显式声明 acked。可执行场景
（cluster-head-wording-exclusivity）的措辞随动不属锁定范围。

## D3 范围边界

- `Used {短名}`（单工具簇头写工具短名）**不在本票**：工具短名是动作归属而非
  用户内容文件，且 att24 对其有独立词形条款；若也要计数式另行开票。
- `basename()` 助手随调用点消亡即删（file_clause 唯一消费者）；`.` / `..`
  卫生条款随计数式天然消失，不再需要特判。
- att35 计数后缀、Ran / Used 分句、att23/26 嵌套与回合窗零改动。

## D4 验证缝

沿用既有缝：summary.rs 纯函数单测（单数/复数/无路径检索裸动词/Edited 头）+
att24 既有可执行场景（SceneBuilder 回放读后改写序列）断言随动；c2510 att35
单文件帧断言随动（`Exploring 1 file · 1 read`）。不新增缝。
