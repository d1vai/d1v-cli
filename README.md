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
