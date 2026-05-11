- pma 是 Rust 编写的 Git 仓库管理工具，支持批量 release、sync、doctor 等操作。
- 每次修改后使用命令格式化
- 修改完成使用下面命令检查有无错误，并修复错误
```shell
   cargo fmt --all --check
   cargo clippy --all -- -D warnings
```
