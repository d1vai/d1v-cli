# Goal: Make D1V Local Runtime A First-Class Managed Runtime

## Current Execution

- Goal: release the tested terminal and container shell workflows through the public CLI distribution channels as `v0.1.24`.
- Background: `main` contains the completed `d1v shell` / `d1v exec` implementation, heartbeat reliability fix, process and relay E2E coverage, and bilingual adoption docs, but the latest downloadable release remains `v0.1.23`.
- Selected validators: `@cli-ux-qa`, `@cli-json-qa`, `@api-backend-qa`, `@auth-state-qa`, `@docs-adoption-qa`, `@session-runtime-qa`, `@deploy-release-qa`.
- Todo:
  - [x] Bump the CLI package and lockfile versions from `0.1.23` to `0.1.24`.
    - Acceptance: Cargo metadata and the compiled `d1v --version` agree on `0.1.24`; no API crate version is changed.
    - Validators: `@deploy-release-qa`, `@migration-compat-qa`.
    - Evidence: Cargo metadata reports `d1v-cli 0.1.24` and unchanged `d1v-api 0.1.7`; the locally compiled binary reports `d1v 0.1.24`.
  - [ ] Run the local release gate before creating the immutable tag.
    - Acceptance: formatting, all-target checks, full workspace tests, publish dry-run, CLI help/JSON output, and shell/exec help all pass on the release commit.
    - Validators: `@cli-ux-qa`, `@cli-json-qa`, `@api-backend-qa`, `@auth-state-qa`, `@docs-adoption-qa`, `@session-runtime-qa`, `@deploy-release-qa`.
    - Evidence: formatting, Cargo metadata, compiled version, all-target checks, and 332 workspace tests passed with one existing ignored test. The first publish dry-run did not start because the host's default `python3` predates the script's required structural pattern matching; rerun with the workflow-compatible Python toolchain.
  - [ ] Push the release commit and signed-off `v0.1.24` tag, then wait for Release and Publish workflows.
    - Acceptance: all seven platform builds succeed, checksums and attestations are attached, the GitHub release is public, and crates.io serves `d1v-cli 0.1.24`.
    - Validators: `@deploy-release-qa`.
  - [ ] Verify the real user installation path and release artifact.
    - Acceptance: the public installer resolves `v0.1.24`; the native macOS artifact passes checksum validation and reports `d1v 0.1.24`; `shell --help` and `exec --help` expose the released command surface.
    - Validators: `@cli-ux-qa`, `@docs-adoption-qa`, `@deploy-release-qa`.

### Validator Handoff

- Result: in progress.
- Checked: the latest public tag is `v0.1.23`; terminal workflows and their CI/E2E gates already pass on `main`; package and lockfile metadata now agree on `0.1.24`.
- Passed: pre-release repository/workflow inspection, release version consistency, formatting, all-target compilation, workspace tests, and command-surface smoke.
- Failed: the first local publish dry-run used an incompatible host Python interpreter and exited before invoking Cargo; corrective rerun is pending with a workflow-compatible interpreter.
- Not checked: clean-tree publish dry-run, GitHub release assets, crates.io publication, and public installer resolution.
- Risk: a tag is immutable deployment input, so it must not be pushed until every local release validator passes.
- Plan update: version bump is the only implementation change in this execution; publication and downstream verification follow it.

## 设计思想与需求背景

### 目标用户与根本诉求

- ICP 是希望把 `d1v.ai` 作为统一控制面、同时把“实际运行机器”放在云端或自己设备上的开发者、团队与内部运营者。
- 当前系统默认把 runtime 等同于 AWS 容器；这限制了高隐私、已有本地代码库、已有本地依赖环境、多设备协作、以及低延迟开发场景。
- 终态目标不是“把本地电脑临时接进现有云容器流程”，而是把 runtime 抽象统一掉：
  - 云 pod 是一种 runtime
  - 用户本地机器上的 `opcode-api` 也是一种 runtime
  - 前端只面向项目与 runtime 能力
  - backend 始终是唯一控制面
  - `d1v-cli` 只是 installer / launcher / supervisor / connector
  - `opcode-api` 才是本地 runtime 的真实服务面

### 终态架构原则

- 统一 runtime contract：cloud/local 都实现同一套 runtime API 与 session/storage/execute contract。
- 统一 project identity：平台主键仍是云端 `UserProject.id`，本地项目通过 binding 关联，不另造第二套平台项目体系。
- 统一 routing：前端永远不直连用户机器；backend 根据 project runtime binding 统一路由 execute / ws / storage / deploy / session。
- 本地项目发现应下沉到 `opcode-api`：CLI 不应长期承担本地项目 catalog 真相。
- 本地能力应以“受控 runtime server”形式存在，而不是“CLI 里堆业务逻辑”。

### 为什么要这样做

- 对用户：
  - 可以把本地机器当成一台真正可管理的 runtime machine
  - 可在本地 home 下创建、导入、发现项目
  - 绑定云端项目后继续复用已有 execute / deploy / session / env / db 等能力
- 对架构：
  - 避免前端和 backend 分别维护 cloud/local 两套项目语义
  - 避免 `d1v-cli` 膨胀成第二个本地应用层
  - 为未来多设备、离线恢复、审计、权限、设备认证、runtime capability negotiation 留出清晰边界

### 关键终态划分

- `opcode-api`
  - 本地 runtime 的真实服务面
  - 负责 project catalog / session catalog / execute / storage / ws / health / capabilities
  - 支持 `standalone` 与 `cloud-managed` 两种模式
- `d1v-cli`
  - 负责 `init-home` / `pair` / `start` / `status`
  - 负责启动、守护、配置、连接本地 `opcode-api`
  - 不负责长期维护项目真相
- `backend_admin`
  - 负责设备、项目、binding、权限、审计、runtime routing、统一 websocket 代理、通知、计费、部署控制
- `d1vai`
  - 负责 devices、runtime binding、local project discovery、runtime-aware session UX
  - 所有交互都走平台 API

### 本轮重写后的执行标准

- 以终态为导向规划，不再把 local runtime 当成临时补丁。
- 所有 Todo 必须：
  - 说明产出
  - 写清验收要求
  - 标注负责的验收员
- 所有实现必须尽量复用既有云端逻辑，而不是复制一套 local-only 流程。

## 验收员定义

- `@runtime-contract-qa`
  - 检查 cloud/local runtime API 是否统一、字段是否稳定、接口语义是否一致。
- `@local-runtime-qa`
  - 检查本地 home、项目发现、path binding、execute/session/storage 在本地 runtime 下是否行为正确。
- `@backend-routing-qa`
  - 检查 backend 是否真正 project-aware runtime routing，且没有遗漏直接走 cloud opcode client 的路径。
- `@frontend-runtime-ux-qa`
  - 检查前端是否能正确展示设备、本地项目、runtime 状态、绑定关系，并形成完整用户闭环。
- `@ops-reliability-qa`
  - 检查 agent start、opcode-api 守护、断线重连、session pinning、离线恢复、错误提示是否可靠。
- `@security-privacy-qa`
  - 检查设备认证、token 使用范围、最小暴露面、敏感信息持久化边界是否符合预期。
- `@migration-compat-qa`
  - 检查向后兼容、旧命令保留、旧 binding 迁移、已有项目不回归。

## Todo List

- [x] 把 local project/session catalog 从 CLI 临时逻辑下沉到 `opcode-api` 原生接口。 `@runtime-contract-qa` `@local-runtime-qa` `@migration-compat-qa`
  - 产出：
    - `opcode-api` 新增本地 runtime catalog API
    - 至少包含 project list / project detail / session list / health / capabilities
    - 明确 `standalone` 与 `cloud-managed` 模式
  - 验收要求：
    - 本地 runtime 不依赖 CLI 扫目录即可返回 home 下项目列表
    - 返回结构足够支撑前端列项目、列 session、做 binding
    - API 语义与云端 runtime 保持兼容，backend 能统一代理
    - 旧 CLI 临时扫描逻辑可保留过渡，但新主路径必须优先走 `opcode-api`

- [x] 为 `opcode-api` 增加 cloud-managed runtime mode。 `@runtime-contract-qa` `@ops-reliability-qa` `@security-privacy-qa`
  - 产出：
    - 启动参数或配置支持 `mode=standalone|cloud-managed`
    - 支持 `runtime-home`、`device-id`、`cloud-control-url`、必要 token/config 注入
  - 验收要求：
    - 本地 `opcode-api` 能在 cloud-managed mode 下启动并上报健康状态
    - 未配置 cloud 管理信息时仍可 standalone 工作
    - 不把用户账户语义硬编码进 `opcode-api`
    - 模式切换边界清晰，日志能识别当前模式

- [x] 把 `d1v-cli` 收敛为 launcher / supervisor / connector。 `@local-runtime-qa` `@ops-reliability-qa` `@migration-compat-qa`
  - 产出：
    - 稳定命令面：`init-home`、`pair`、`start`、`status`、`agent project ...`
    - CLI 对本地项目发现改为调用本地 `opcode-api`
    - 启动与守护逻辑清晰，配置文件 schema 明确
  - 验收要求：
    - `d1v agent start` 可在 opcode-api 未运行时拉起并接入 backend
    - token 过期、端口冲突、home 不存在、binary 缺失等错误可解释
    - 兼容命令 `init-runtime` 保留但明确标记为兼容入口
    - CLI 不再承担 project catalog 的 source-of-truth 角色

- [x] 在 `backend_admin` 建立统一 runtime router，替换零散的 direct opcode client 路径。 `@backend-routing-qa` `@runtime-contract-qa` `@migration-compat-qa`
  - 产出：
    - 统一 `get_project_runtime_client` / runtime router 抽象
    - execute / cancel / session ws / storage / model config / deploy / git ops 等项目级能力全部 project-aware
  - 验收要求：
    - 不再存在关键项目路径绕过 runtime binding 直接打到 cloud opcode
    - 创建项目、导入项目、仓库迁移等 provisioning 阶段允许保留 cloud opcode 直连，因为此时项目 runtime binding 尚未形成；这些路径不计入 steady-state router 漏项
    - local runtime 与 cloud runtime 都能复用同一条项目级控制路径
    - runtime 切换只影响新 session，旧 session 固定在原 runtime
    - `WorkerRuntime` 或等价记录里能识别 runtime_type / device_id / session pinning 元数据

- [x] 完成 device home 与 local project binding 的正式模型与迁移收口。 `@backend-routing-qa` `@migration-compat-qa`
  - 产出：
    - device 上的 `runtime_home`
    - project binding 上的 `local_project_path`
    - 明确 binding mode 与 sync policy 的后续扩展位
  - 验收要求：
    - device home 与 project path 不再语义混淆
    - 同设备多项目、本地路径改变、目录删除、重复绑定等场景有稳定行为
    - 旧字段兼容可读，迁移后老数据不导致 execute 回归

- [x] 在前端补齐“本地项目发现 -> 创建/绑定云端项目 -> 切换 runtime -> execute”的完整闭环。 `@frontend-runtime-ux-qa` `@backend-routing-qa` `@local-runtime-qa`
  - 产出：
    - Devices 页面可查看在线设备、device home、本地项目列表
    - Project runtime binding UI 可选择设备与本地项目
    - 支持从本地发现项创建云端项目并自动绑定
    - 支持把本地发现项绑定到已有云端项目
  - 验收要求：
    - 用户不需要手敲命令即可在前端完成常见绑定流程
    - 绑定后继续使用现有项目页 chat/execute/session/deploy 逻辑
    - 本地项目未绑定、设备离线、path 缺失、session 固定在旧 runtime 等情况有明确提示
    - 不新增第二套 local-only 项目 UX

- [x] 把 local session catalog 接入前端现有 session/history/reconnect 逻辑。 `@runtime-contract-qa` `@frontend-runtime-ux-qa` `@backend-routing-qa`
  - 产出：
    - 本地 runtime 的 session list / session detail 可经 backend 暴露
    - 前端 session status / history / reconnect 尽量复用现有组件
  - 验收要求：
    - 本地 runtime 下 active session、history、cancel、ws reconnect 可用
    - session 展示不要求用户理解底层 runtime 来源
    - reconnect 不因 runtime 是 local 而走第二套前端协议

- [x] 完成 local runtime execute 端到端 smoke 与 websocket tunnel 端到端验收。 `@local-runtime-qa` `@ops-reliability-qa` `@backend-routing-qa`
  - 产出：
    - fake agent / fake local runtime 或真实本地 runtime 的 E2E 测试
    - 覆盖 execute -> ws -> result -> cancel -> reconnect
  - 验收要求：
    - 至少有一条可重复自动化跑通的 local runtime 会话测试
    - websocket open/send/close、binary/text、session completion、cancel 都被覆盖
    - 设备断线、backend 重连、runtime 重启后行为可验证

- [x] 为 local runtime 补齐可靠性与恢复策略。 `@ops-reliability-qa` `@backend-routing-qa`
  - 产出：
    - agent relay 重连
    - opcode-api 健康检查与自动拉起策略
    - session pinning 与 runtime offline fallback 规则
  - 验收要求：
    - 本地机器休眠、网络切换、backend 重启后，agent 能恢复连接
    - 运行中的 session 不会因 runtime 切换被错误迁移
    - local runtime 离线时，平台能给出明确错误，而不是静默失败
    - 新 session 是否允许 fallback 到 cloud 必须有明确策略，且默认值固定

- [x] 为 local runtime 补齐安全与隐私边界。 `@security-privacy-qa` `@ops-reliability-qa`
  - 产出：
    - 设备公私钥或等价设备认证机制
    - backend 只接受已配对 device 连接
    - 最小持久化原则与审计边界说明
  - 验收要求：
    - 前端不直连用户机器
    - 本地项目内容不因 relay 被不必要持久化到平台
    - token、device identity、pairing code 生命周期有明确边界
    - 高隐私模式下哪些内容仍会进平台数据库必须明确

- [x] 统一文档、README、架构文档与 PLAN 的表述，避免 local runtime 被理解成“CLI 魔法功能”。 `@migration-compat-qa` `@frontend-runtime-ux-qa`
  - 产出：
    - README / docs / 架构文档统一说明：
      - `opcode-api` 是本地 runtime server
      - `d1v-cli` 是启动与接入器
      - backend 是控制面
    - 用户流程文档覆盖：
      - init-home
      - pair
      - start
      - create/import/bind
      - 前端绑定与 execute
  - 验收要求：
    - 新同学仅看文档即可理解四层职责边界
    - 文档中的命令面、页面路径、接口名称与实现一致
    - 不再把 CLI 扫目录的过渡逻辑描述成长期架构

- [x] 建立 `opcode-api` 私有源码、公开二进制资产的 runtime 分发链路。 `@ops-reliability-qa` `@security-privacy-qa` `@migration-compat-qa`
  - 产出：
    - `opcode-api` release packaging 脚本与 runtime manifest workflow
    - `d1v-cli` runtime installer 优先读取 manifest/CDN 资产，避免依赖公开源码仓库 release 页面
    - manifest 支持 target/url/sha256，便于将来切到对象存储或 CDN
  - 验收要求：
    - 用户安装 runtime 时不需要访问 `opcode-api` 源码仓库
    - 公网仅暴露二进制包、manifest、checksum，不暴露源码与提交历史
    - CLI 在 manifest 显式配置时不再静默 fallback 到错误的公开源

- [x] 提升 `d1v-cli` 本地 runtime 安装与接入体验。 `@local-runtime-qa` `@ops-reliability-qa` `@frontend-runtime-ux-qa`
  - 产出：
    - `scripts/install-opcode-runtime.sh` 安装引导脚本
    - `agent start` 本地 health check / 端口冲突 / 日志路径 / 版本提示优化
    - `agent init-home` 从必须登录改为纯本地可执行
  - 验收要求：
    - 用户可在未登录状态先初始化本地 home
    - 本地 loopback health/relay 不受代理环境变量污染
    - runtime 启动失败时能明确提示日志位置和冲突端口

- [x] 为安装、代理接入、cloud/local runtime 补自动化 workflow 与脚本化 E2E。 `@ops-reliability-qa` `@backend-routing-qa` `@local-runtime-qa`
  - 产出：
    - `d1v-cli` runtime install + relay attach workflow
    - root backend cloud/local container E2E workflow
    - 本地 agent relay E2E 脚本与 local runtime execute 回归测试
  - 验收要求：
    - 安装 runtime、初始化 home、agent relay attach、项目 execute 都有自动化覆盖
    - cloud container smoke 与 local container smoke 都能独立回归
    - 新增测试能覆盖代理环境、端口冲突、local runtime execute 主路径

## 默认决策与实现假设

- 平台项目主键继续使用云端 `UserProject.id`。
- local project discovery 是 runtime 能力，不是平台第二套项目体系。
- 前端永远不直连用户机器。
- runtime 切换默认只影响新 session。
- local runtime 发现、session list、execute、storage、ws 都应优先通过 `opcode-api` 暴露。
- `d1v-cli` 中已存在的临时项目发现逻辑只作为过渡兼容，不作为终态设计。
- 短期不引入复杂 ZKP；优先设备认证、最小暴露面、受控 relay、审计与清晰隐私边界。

## 近期执行顺序

1. `opcode-api` 接手 local project/session catalog。
2. `backend_admin` 完成统一 runtime router。
3. `d1v-cli` 收敛为 launcher/supervisor/connector。
4. `d1vai` 补完整绑定与执行闭环。
5. 补 E2E 与可靠性、安全验收。

## Public Expose Extension

### 目标

为 `d1v-cli` 增加最小的公网暴露命令面：

- `d1v expose 3000`
- `d1v expose list`
- `d1v expose close <binding_id>`

命令目标不是管理 Cloudflare，而是让用户拿到：

- `https://abc123.cli-free.d1v.dev`

### CLI 设计原则

- CLI 不持有 Cloudflare token。
- CLI 不直接创建 DNS 记录。
- CLI 只调用平台控制面或本地 runtime-agent 管理接口。
- 节点域名分配必须由后端返回。

### TODO 13: 新增 `d1v expose` 命令面

- 在 `d1v-cli` 新增：
  - `d1v expose <port>`
  - `d1v expose list`
  - `d1v expose close <binding_id>`
- 最小参数：
  - `--project-id`
  - `--hostname`

预期验证结果：

- 用户执行 `d1v expose 3000` 后，CLI 直接打印最终域名。
- list/close 能复用同一份 expose binding 真相。

### TODO 14: 节点身份与域名提示

- `d1v agent start` 成功后，如果设备注册为平台节点：
  - 打印 `public_origin`
  - 打印对应 `xxx.node.d1v.dev`
- `d1v expose` 成功后，必须回显：
  - `binding_id`
  - `public_url`
  - `target_port`

预期验证结果：

- 用户不需要额外去后台查域名。
- 节点级域名和 expose 级域名都能在 CLI 第一时间看到。

### TODO 15: 节点启动后的 expose supervisor 规划

- `runtime-agent` 启动后，需要常驻一个后台线程：
  - 读取本节点当前活跃容器端口
  - 拉取后端分配给本节点的 expose bindings
  - 更新本地 ingress 路由表
- CLI 文档与命令帮助要明确：
  - `d1v expose` 的真实生效依赖节点 supervisor
  - expose 并非一次性 shell 脚本，而是长期托管状态

预期验证结果：

- 节点重启后，原 expose route 能恢复。
- 容器端口变化后，公开域名仍可访问。

### TODO 16: 浏览器终端与 expose 分层

- CLI 文档明确：
  - `d1v expose` 只负责 Web 服务暴露
  - 浏览器 terminal 继续走平台 relay
  - 后续若做 `d1v shell`，应作为独立命令面

预期验证结果：

- 命令面不会把“公开预览”和“容器终端”混成一件事。
- 产品路径更容易扩展权限与审计。
