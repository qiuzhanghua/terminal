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

- **Windows**: 自动检测可用 shell（优先级：pwsh → powershell → cmd）
- **非Windows**: 检测用户缺省 shell（读取 `$SHELL` 环境变量，默认 `/bin/bash`），以子进程方式进入
- 支持管道：`ls -la | grep rust`
- 支持重定向：`echo hello > file.txt`, `cat < input.txt`
