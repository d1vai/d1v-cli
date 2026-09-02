<div align="center">

# D1V CLI

[d1v.ai](https://www.d1v.ai/) 的项目部署、远程终端与 AI coding 命令行工具。

[![CI](https://github.com/d1vai/d1v-cli/actions/workflows/ci.yaml/badge.svg)](https://github.com/d1vai/d1v-cli/actions/workflows/ci.yaml)
[![Release](https://img.shields.io/github/v/release/d1vai/d1v-cli?display_name=tag)](https://github.com/d1vai/d1v-cli/releases)
[![crates.io](https://img.shields.io/crates/v/d1v-cli)](https://crates.io/crates/d1v-cli)
[![Homebrew](https://img.shields.io/homebrew/v/d1v?tap=d1vai%2Ftap)](https://github.com/d1vai/homebrew-tap)
[![License](https://img.shields.io/github/license/d1vai/d1v-cli)](LICENSE)
[![Stars](https://img.shields.io/github/stars/d1vai/d1v-cli)](https://github.com/d1vai/d1v-cli/stargazers)
[![Rust](https://img.shields.io/badge/rust-1.95%2B-orange)](https://www.rust-lang.org/)

默认识别当前项目，支持 Codex、Claude Code、云端 Runtime 与本地 Runtime。
<samp>

**[English](README.md)** ┃ **[简体中文](README.zh-Hans.md)**

</samp>

</div>

## 命令

运行 `d1v --help` 查看所有可用命令。

在当前目录快速部署，无需重复填写项目 ID：

```sh
curl -fsSL https://d1v.ai/install/d1v-cli.sh | bash
d1v auth login --browser
d1v --preview   # 或：d1v --prev
d1v --prod
```

CLI 会优先从当前目录的 `.env` 读取 `D1V_PROJECT_ID`，然后回退到进程环境变量和
`.d1v/project.json`。显式 project ID 始终优先；交互终端无法解析时可从项目选择器中选择。
随后云端环境变量会合并到 `.env`，冲突时默认保留本地值。

## 编程 Agent Skill

官方且可版本化的 d1v Skill 位于
[`skills/d1v/SKILL.md`](skills/d1v/SKILL.md)。它为 Codex 和 Claude Code
提供项目工作区、容器命令、Preview 部署和经用户确认的生产发布操作指引。

curl 安装器默认使用 `--install-skill auto`：它只会为 `PATH` 中已经存在的
Codex 和 Claude Code 安装 Skill，不会安装这两个 Agent；若都未检测到则不写入
任何文件并正常成功。手动执行时也建议使用：

```sh
d1v skill install --agent auto
```

需要明确指定目标时可使用 `--agent codex`、`--agent claude` 或 `--agent all`。
Skill 分别写入 `${CODEX_HOME:-~/.codex}/skills/d1v` 和
`${CLAUDE_CONFIG_DIR:-~/.claude}/skills/d1v`。内容相同不会重写；已有不同内容
时会先在同一目录备份为 `SKILL.md.d1v-backup-<UTC timestamp>`，再替换。旧地址
`https://www.d1v.ai/cli-skill.md` 保持可用，并会重定向到这份官方文件。

## D1V 移动端

配合 [`d1vai_app`](https://github.com/d1vai/d1vai_app) 使用 D1V。它是 d1v.ai
官方开源 Flutter 客户端：CLI 适合本地开发和 CI，移动端适合在离开电脑时创建项目、
继续 AI 会话、查看文件、监控部署和管理工作区。

[![d1vai_app Stars](https://img.shields.io/github/stars/d1vai/d1vai_app?label=d1vai_app%20stars)](https://github.com/d1vai/d1vai_app/stargazers)
[![Flutter](https://img.shields.io/badge/Flutter-3.x-02569B?logo=flutter&logoColor=white)](https://flutter.dev/)
[![Mobile](https://img.shields.io/badge/mobile-iOS%20%7C%20Android-16a34a)](https://github.com/d1vai/d1vai_app/releases)

移动端支持 iOS 和 Android，并提供截图、发布下载和开发者指南：

- [d1vai_app 仓库](https://github.com/d1vai/d1vai_app)
- [移动端发布版](https://github.com/d1vai/d1vai_app/releases)
- [d1v.ai 文档](https://www.d1v.ai/docs/overview)

## 本地 Runtime

D1V 现在同时支持云端 runtime 和本地 runtime。

职责划分：

- `opcode-api`：本地 runtime server
- `d1v-cli`：安装器、启动器、守护器、接入器
- `backend_admin`：控制面与 runtime 路由器
- `d1vai`：统一前端

前端不会直连用户机器。本地 agent 会主动向 D1V 云端建立出站连接。

### 本地 Runtime 快速开始

1. 安装并检查本地 runtime：

```sh
d1v runtime doctor
bash scripts/install-opcode-runtime.sh --home ~/d1v-home
```

2. 绑定设备：

```sh
d1v agent pair
```

或者继续使用 Web UI 里生成的 pairing code：

```sh
d1v agent pair --code <pairing-code>
```

3. 启动本地 runtime：

```sh
d1v agent start
```

4. 创建或绑定本地项目目录：

```sh
d1v agent project create --project-id <project_id> --name my-app
d1v agent project import --path ~/work/my-app --project-id <project_id>
d1v agent project bind --project-id <project_id> --path ~/work/my-app
```

兼容入口：

```sh
d1v agent init-runtime --project-id <project_id> --path ~/work/my-app
```

### 本地 Runtime 行为规则

- runtime 切换只影响新 session。
- 已经开始的 session 会固定在原 runtime 上。
- 如果项目绑定到本地 runtime，但设备离线，平台会返回明确错误，不会静默回退到云端。
- 创建项目、导入项目这类 runtime binding 尚未形成的 provisioning 流程，仍可能直接走 cloud opcode。

### Public Expose

CLI 免费 expose 不需要运行 node agent：

```sh
d1v expose 3000
d1v expose list
d1v expose close <binding_id>
```

平台节点 ingress 使用独立的命令路径：

```sh
d1v node expose 3000 --node-id <platform-node-id>
d1v node expose list
d1v node expose close <binding_id>
```

当前 expose 模式：

- `cli_free_relay`：为已登录 CLI 的临时 relay 返回公开的 `https://*.cli-free.d1v.dev` URL
- `cloudflare_tunnel`：为平台节点返回公开的 `https://*.node.d1v.dev` URL
- `reverse_relay`：CLI 免费入口内部用于本地/用户 relay 的回退模式

`d1v expose` 当前只面向 HTTP 服务。浏览器 terminal 和 session WebSocket 仍然继续走现有后端 relay。

### 平台节点

平台节点现在使用单一 control origin 作为 runtime-agent 入口。推荐启动方式：

```sh
d1v node start --key <platform-node-key> --control-origin https://{your-host-or-name}-node.d1v.dev
```

如果不传 `--control-origin`，runtime-agent 会继续走公网 IP 检测和控制面注册流程。

### 隐私边界

当前阶段：

- 只允许已配对设备
- 仅出站 relay
- pairing code 短时有效
- 平台保存设备 public key

当前限制：

- relay 链路还不是零知识
- 平台侧仍会按现有 D1V 逻辑持久化必要的 metadata 与 chat/session 记录

更完整的架构说明见 [docs/d1v-agent-architecture.md](../docs/d1v-agent-architecture.md)。

## 安装

推荐方式：

```sh
curl -fsSL https://d1v.ai/install/d1v-cli.sh | bash
```

安装页面：

```sh
https://d1v.ai/cli-install
```

备选方式：

```sh
brew install d1vai/tap/d1v
cargo binstall d1v-cli
cargo install --locked d1v-cli
```

安装后：

```sh
d1v auth login
d1v project list
d1v github status
```

如需复用已有浏览器登录状态并保存一个可撤销的设备 API key，可使用
`d1v auth login --browser`。浏览器只批准一个有效期 10 分钟的一次性会话；CLI 会将
生成的 key 保存到 `~/.d1v/config.toml`。

后续升级：

```sh
d1v upgrade
d1v upgrade --version v0.1.5
d1v uninstall
```

`d1v upgrade` 只替换可执行文件。登录凭据保存在 `~/.d1v/config.toml`，升级不会退出登录，也不会把凭据复制到发布包中。

### 全局选项

| 选项       | 描述                  | 默认值      |
| ---------- | --------------------- | ----------- |
| `--format` | 输出格式 (text, json) | text        |
| `--color`  | 颜色输出              | auto        |
| `--lang`   | 显示语言              | 系统 / 配置 |
| `-v`       | 提高日志详细程度      | warn        |

### 环境变量

| 变量              | 描述          |
| ----------------- | ------------- |
| `D1V_API_KEY`     | API 密钥      |
| `D1V_AUTH_TOKEN`  | 认证令牌      |
| `D1V_BASE_URL`    | API 基础地址  |
| `D1V_FORMAT`      | 输出格式      |
| `D1V_LANG`        | 显示语言      |
| `D1V_LOG_FILE`    | 日志文件路径  |
| `D1V_RECORD_FILE` | HTTP 录制文件 |
| `NO_COLOR`        | 禁用颜色输出  |
| `RUST_LOG`        | 日志过滤器    |

### 认证

| 命令              | 描述         |
| ----------------- | ------------ |
| `d1v auth login`  | 登录         |
| `d1v auth logout` | 退出登录     |
| `d1v auth status` | 查看认证状态 |

### 配置

| 命令               | 描述                 |
| ------------------ | -------------------- |
| `d1v config show`  | 显示当前配置         |
| `d1v config get`   | 获取配置项值         |
| `d1v config set`   | 设置配置项值         |
| `d1v config list`  | 列出所有配置项       |
| `d1v config path`  | 显示配置文件路径     |
| `d1v config reset` | 将配置重置为默认值   |
| `d1v config edit`  | 用编辑器打开配置文件 |

可用配置项：

| 键         | 描述         |
| ---------- | ------------ |
| `base_url` | API 基础地址 |
| `language` | 显示语言覆盖 |

### 用户

| 命令              | 描述             |
| ----------------- | ---------------- |
| `d1v user info`   | 查看当前用户信息 |
| `d1v user update` | 更新用户信息     |
| `d1v user get`    | 查看公开用户     |
| `d1v user list`   | 列出所有用户     |

### 密码

| 命令                      | 描述     |
| ------------------------- | -------- |
| `d1v user password set`   | 设置密码 |
| `d1v user password reset` | 重置密码 |

### 邮箱

| 命令                    | 描述     |
| ----------------------- | -------- |
| `d1v user email bind`   | 绑定邮箱 |
| `d1v user email change` | 更换邮箱 |

### 邀请与引导

| 命令                         | 描述         |
| ---------------------------- | ------------ |
| `d1v user invitation accept` | 接受邀请     |
| `d1v user invitation list`   | 查看邀请列表 |
| `d1v user onboard`           | 标记引导完成 |

### 活动统计

| 命令                | 描述             |
| ------------------- | ---------------- |
| `d1v user activity` | 查看每日活动统计 |

### 诊断

| 命令        | 描述         |
| ----------- | ------------ |
| `d1v debug` | 显示调试信息 |
| `d1v upgrade` | 检查更新并自升级 |
| `d1v uninstall` | 移除当前 d1v 可执行文件 |

## 项目工作流

以下命令都依赖登录态，建议先执行：

```sh
d1v auth status
d1v auth login
```

### 核心资源

| 领域 | 能力 | 从这里开始 |
| --- | --- | --- |
| 项目 | 列出、查看、创建、更新、删除项目 | `d1v project list` |
| 会话 | 启动或继续 AI coding 会话 | `d1v session run ...` |
| 部署 | Preview、生产发布、状态、历史、日志 | `d1v --prev` |
| 容器 | 打开 PTY 或执行单条命令 | `d1v shell` / `d1v exec -- ...` |
| 环境变量 | 查看、设置、导入、导出、同步变量 | `d1v env list` |
| 数据库 | 查看 schema/data，管理迁移 | `d1v db schema ...` |
| GitHub | 绑定仓库、导入项目 | `d1v github status` |
| Expose | 临时公开本地 HTTP 端口 | `d1v expose 3000` |
| Runtime | 配对并运行本地 Runtime | `d1v agent pair` |

所有项目级命令使用统一的项目解析顺序：

```text
显式 project ID
-> 当前目录 .env: D1V_PROJECT_ID
-> D1V_PROJECT_ID 环境变量
-> workspace binding
```

### 容器终端与命令执行

可以进入当前 `.env` 指定的项目目录：

```sh
d1v shell
d1v shell <project_id>
d1v shell --workspace
d1v shell --workspace --organization-id <organization_id>
```

`d1v shell` 会优先读取当前目录 `.env`。需要明确进入 workspace 根目录时使用 `--workspace`；位置参数中的项目 ID 始终优先。

交互终端要求 TTY，并直接使用容器内 Bash/Zsh 的原生自动补全。Agent、CI 以及需要捕获输出或退出状态的任务应使用非交互 `d1v exec`，把 argv 放在 `--` 后传入：

```sh
d1v exec -- pwd
d1v exec --project-id <project_id> -- npm test
d1v exec --workspace -- pwd
d1v exec --workspace --organization-id <organization_id> -- pwd
d1v --format json exec --project-id <project_id> -- sh -c 'printf ok; printf problem >&2; exit 7'
```

不传 `--project-id` 时，`d1v exec` 会从当前 `.env` 解析项目；需要 workspace 根目录时使用 `--workspace`。

文本模式会把远端 stdout、stderr 分别流式写到本地对应输出流。JSON 模式稳定返回 `session_id`、`project_id`、`cwd`、`exit_code`、`stdout` 和 `stderr`，同时 CLI 进程会保留远端的非零退出码。交互式 shell 不支持 JSON 输出。

CLI 会自动选择符合条件的 direct-node 连接，否则回退到后端 relay；每 20 秒发送一次应用层 heartbeat，避免长时间 session 被中间网络设备误清理。Shell ticket 有效期很短，只通过 WebSocket header 发送，不会进入 URL 或命令输出。终端服务不会持久化终端输入和输出内容。

针对项目启动 AI coding 会话并选择引擎：

```sh
d1v session run <project_id> --engine codex --prompt "运行测试并修复失败"
d1v session run <project_id> --engine claude --prompt "审查认证流程"
```

`--engine` 可以省略，后端会根据模型推断；需要可复现时建议显式指定。

### 容器内按需启用集成

当容器运行时注入了 `D1V_API_KEY`（或 `D1V_AUTH_TOKEN`）、`D1V_BASE_URL`、`D1V_PROJECT_ID` 之后，agent 可以直接按需启用项目集成，不需要再走浏览器登录：

```sh
d1v project ensure database
d1v project ensure db analytics
d1v --format json project ensure pay
```

### GitHub 跳转路径

优先走 CLI，只有在绑定或安装缺失时再跳浏览器：

```sh
d1v github status
d1v github bind
d1v github installations
d1v github repos --installation-id 123456
```

如果 GitHub App 安装或 OAuth 绑定还没完成，`d1v github bind` 会打开正确的页面，包括需要时跳到 `https://d1v.ai/setting?tab=github`。

### 数据库与迁移最小 Smoke 清单

登录并拿到项目 id 后，可以按这条最短路径验证数据库链路：

```sh
d1v db token issue <project_id> --scopes db:read,migrate
d1v db schema <project_id>
d1v db rows list <project_id> --schema public --table your_table
d1v db migrate plan <project_id> --sql 'CREATE TABLE IF NOT EXISTS smoke_cli(id serial primary key);'
```

后续常用命令：

```sh
d1v db migrate validate <plan_id>
d1v db migrate approve <plan_id>
d1v db migrate auto-review <approval_id>
```

## 开发

### 前置要求

- [Rust](https://www.rust-lang.org/)（稳定版 1.95+）
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

日志按天写入 `~/.d1v/d1v.YYYY-MM-DD.log`，保留最近 7 天。

使用 `-v` 提高 stderr 日志详细程度（`-v` info，`-vv` debug，`-vvv` trace）：

```sh
d1v -vv auth login
```

未使用 `-v` 时也支持 `RUST_LOG`：

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

启用 `record` 特性后可用的配置项：

| 键               | 描述             |
| ---------------- | ---------------- |
| `record.enabled` | 默认启用录制     |
| `record.dir`     | 录制文件保存目录 |
