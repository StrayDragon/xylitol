# session-tree

产品树是 Driver MessageHistory **活树**（双 Esc 开）。kind 前缀（user/assistant/tool）走 token 色；选中反转 inherit，禁止 kind 前景穿透。

- filter 时状态 `(n/m)` 出现在选中行之后。
- Enter 经 travel：user 叶预填 input；Esc 关树。
- 替换 editor 槽，不是居中 overlay。
