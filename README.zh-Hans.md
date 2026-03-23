<div align="center">

# D1V CLI

[d1v.ai](https://www.d1v.ai/) 实验性命令行工具。
<br><br>
<a href="https://github.com/fhluo/d1v-cli/actions/workflows/ci.yaml">
<img src="https://github.com/fhluo/d1v-cli/actions/workflows/ci.yaml/badge.svg" alt="ci workflow"></a>

<samp>

**[English](README.md)** ┃ **[简体中文](README.zh-Hans.md)**

</samp>

</div>

## 命令

运行 `d1v --help` 查看所有可用命令。

### 认证

| 命令              | 描述     |
| ----------------- | -------- |
| `d1v auth login`  | 登录     |
| `d1v auth logout` | 退出登录 |

### 用户

| 命令              | 描述             |
| ----------------- | ---------------- |
| `d1v user info`   | 查看当前用户信息 |
| `d1v user update` | 更新用户信息     |
| `d1v user get`    | 查看公开用户     |
| `d1v user list`   | 列出所有用户     |

### 诊断

| 命令        | 描述         |
| ----------- | ------------ |
| `d1v debug` | 显示调试信息 |

## 开发

### 前置要求

- [Rust](https://www.rust-lang.org/)（最新稳定版）
- [Task](https://taskfile.dev/)（可选）

### 构建

```sh
cargo build
```

### 测试

```sh
cargo test
```

### 运行

```sh
cargo run
```

### 安装

```sh
cargo install --path d1v-cli
```

## 调试

### 环境信息

检查版本、配置路径和 Token 状态：

```sh
d1v debug
```

### 日志

日志默认写入 `~/.d1v/d1v.log`。

输出调试日志到 stderr：

```sh
RUST_LOG=debug d1v auth login
```

写入日志到指定文件：

```sh
d1v --log-file trace.log auth login
```

### HTTP 录制

将 HTTP 流量保存为 JSON 文件以供排查。

启用 `record` 特性：

```sh
cargo install --path d1v-cli --features record
```

运行并录制：

```sh
d1v --record trace.json auth login
```
