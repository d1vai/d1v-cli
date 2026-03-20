<div align="center">

# D1V CLI

Experimental CLI for [d1v.ai](https://www.d1v.ai/).
<br><br>
<a href="https://github.com/fhluo/d1v-cli/actions/workflows/ci.yaml">
<img src="https://github.com/fhluo/d1v-cli/actions/workflows/ci.yaml/badge.svg" alt="ci workflow"></a>

<samp>

**[English](README.md)** ┃ **[简体中文](README.zh-Hans.md)**

</samp>

</div>

## Commands

Run `d1v --help` for all available commands.

### Authentication

| Command           | Description |
| ----------------- | ----------- |
| `d1v auth login`  | Log in      |
| `d1v auth logout` | Log out     |

### Diagnostics

| Command     | Description            |
| ----------- | ---------------------- |
| `d1v debug` | Show debug information |

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable)
- [Task](https://taskfile.dev/) (optional)

### Build

```sh
cargo build
```

### Test

```sh
cargo test
```

### Run

```sh
cargo run
```

### Install

```sh
cargo install --path d1v-cli
```

## Debugging

### Environment

Check version, config path, and token status:

```sh
d1v debug
```

### Logging

Logs are written to `~/.d1v/d1v.log` by default.

Print debug logs to stderr:

```sh
RUST_LOG=debug d1v auth login
```

Write logs to a custom file:

```sh
d1v --log-file trace.log auth login
```

### HTTP Recording

Capture HTTP traffic to a JSON file for inspection.

Enable the `record` feature:

```sh
cargo install --path d1v-cli --features record
```

Run with recording:

```sh
d1v --record trace.json auth login
```
