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

## 本地 Runtime

D1V 现在同时支持云端 runtime 和本地 runtime。

职责划分：

- `opcode-api`：本地 runtime server
- `d1v-cli`：安装器、启动器、守护器、接入器
- `backend_admin`：控制面与 runtime 路由器
- `d1vai`：统一前端

前端不会直连用户机器。本地 agent 会主动向 D1V 云端建立出站连接。

### 本地 Runtime 快速开始

1. 初始化 runtime home：

```sh
d1v agent init-home --path ~/d1v-home
```

2. 在 Web UI 生成 pairing code，然后在本机执行：

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
brew install d1v-ai/tap/d1v
cargo binstall d1v-cli
cargo install --locked d1v-cli
```

安装后：

```sh
d1v auth login
d1v project list
d1v github status
```

后续升级：

```sh
d1v upgrade
d1v upgrade --version v0.1.4
d1v uninstall
```

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

### 核心资源命令

| 领域      | 命令 |
| --------- | ---- |
| 项目      | `d1v project list|get|create|update|delete|templates` |
| 会话      | `d1v session run|continue|status|history|cancel` |
| 部署      | `d1v deploy preview|prod|status|history|logs` |
| GitHub    | `d1v github status|bind|installations|repos|import` |
| 数据库    | `d1v db schema|data|branches|tables|rows|token|migrate` |

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
