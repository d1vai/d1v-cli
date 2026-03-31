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

### Global Options

| Option      | Description                | Default         |
| ----------- | -------------------------- | --------------- |
| `--format`  | Output format (text, json) | text            |
| `--color`   | Color output               | auto            |
| `--lang`    | Display language           | System / Config |
| `-v`        | Increase log verbosity     | warn            |

### Authentication

| Command           | Description |
| ----------------- | ----------- |
| `d1v auth login`  | Log in      |
| `d1v auth logout` | Log out     |

### User

| Command           | Description            |
| ----------------- | ---------------------- |
| `d1v user info`   | Show current user info |
| `d1v user update` | Update user info       |
| `d1v user get`    | Get a public user      |
| `d1v user list`   | List all users         |

### Password

| Command                   | Description    |
| ------------------------- | -------------- |
| `d1v user password set`   | Set a password |
| `d1v user password reset` | Reset password |

### Email

| Command                 | Description   |
| ----------------------- | ------------- |
| `d1v user email bind`   | Bind an email |
| `d1v user email change` | Change email  |

### Invitation & Onboarding

| Command                      | Description                 |
| ---------------------------- | --------------------------- |
| `d1v user invitation accept` | Accept an invitation        |
| `d1v user invitation list`   | List invited users          |
| `d1v user onboard`           | Mark onboarding as complete |

### Activity

| Command             | Description                |
| ------------------- | -------------------------- |
| `d1v user activity` | View prompt daily activity |

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

Daily log files are written to `~/.d1v/d1v.YYYY-MM-DD.log`, keeping the last 7 days.

Increase stderr verbosity with `-v` (`-v` info, `-vv` debug, `-vvv` trace):

```sh
d1v -vv auth login
```

`RUST_LOG` is also supported when `-v` is not set:

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
