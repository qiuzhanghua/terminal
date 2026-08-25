# Terminal

一个交互式 Shell 程序，支持管道、重定向和常用内置命令。

## 运行

```bash
cargo run
```

## 构建

```bash
cargo build
```

## 测试

```bash
cargo test
```

## Lint

```bash
cargo clippy
```

## 功能

- 交互式 shell（方向键历史、自动补全）
- 内置命令：`cd`, `pwd`, `echo`, `exit`, `env`, `set`, `export`, `unset`
- 管道：`ls -la | grep rust`
- 重定向：`echo hello > file.txt`, `cat < input.txt`
