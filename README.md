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

| Option     | Description                | Default         |
| ---------- | -------------------------- | --------------- |
| `--format` | Output format (text, json) | text            |
| `--color`  | Color output               | auto            |
| `--lang`   | Display language           | System / Config |
| `-v`       | Increase log verbosity     | warn            |

### Environment Variables

| Variable          | Description          |
| ----------------- | -------------------- |
| `D1V_AUTH_TOKEN`  | Auth token           |
| `D1V_BASE_URL`    | API base URL         |
| `D1V_FORMAT`      | Output format        |
| `D1V_COLOR`       | Color output         |
| `D1V_LANG`        | Display language     |
| `D1V_LOG_FILE`    | Log file path        |
| `D1V_RECORD_FILE` | HTTP recording file  |
| `NO_COLOR`        | Disable color output |
| `RUST_LOG`        | Log filter           |

### Authentication

| Command           | Description      |
| ----------------- | ---------------- |
| `d1v auth login`  | Log in           |
| `d1v auth logout` | Log out          |
| `d1v auth status` | Show auth status |

### Configuration

| Command            | Description                     |
| ------------------ | ------------------------------- |
| `d1v config show`  | Show current configuration      |
| `d1v config get`   | Get a config value              |
| `d1v config set`   | Set a config value              |
| `d1v config list`  | List available config keys      |
| `d1v config path`  | Print config file path          |
| `d1v config reset` | Reset configuration to defaults |
| `d1v config edit`  | Open config file in editor      |

Available config keys:

| Key        | Description               |
| ---------- | ------------------------- |
| `base_url` | API base URL              |
| `language` | Display language override |

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

Config keys available with the `record` feature:

| Key              | Description                             |
| ---------------- | --------------------------------------- |
| `record.enabled` | Enable recording by default             |
| `record.dir`     | Directory to write recording files into |
