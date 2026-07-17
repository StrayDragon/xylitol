# language: zh-CN
# managed by llman sdd partition-migrate
功能: build-config

  @req:r33
  场景: all-features-on
    假如 Cargo.toml 存在且 default 列表已更新
    当 无额外 flags 运行 cargo build
    那么 二进制包含所有非 dev feature 功能

  @req:r33
  场景: minimal-build
    假如 Cargo.toml 存在且 default 列表已更新
    当 cargo build --no-default-features --features tui 运行
    那么 仅编译 tui 及其依赖
