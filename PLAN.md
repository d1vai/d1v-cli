# Goal: Browser login exchanges for revocable CLI device API keys

## Current Execution

- Goal: implement `d1v auth login --browser` so browser authentication issues a
  long-lived, revocable device API key without placing the key or polling secret
  in a URL, normal output, or logs.
- Background: browser JWTs are not suitable durable CLI credentials. The backend
  already stores hashed user API keys and the CLI already persists them through
  the keyring/config token chain.
- Selected validators: `@cli-ux-qa`, `@cli-json-qa`, `@api-backend-qa`,
  `@auth-state-qa`, `@docs-adoption-qa`.
- Todo:
  - [x] Add short-lived, hashed CLI login sessions and atomic device-key rotation
    to the backend, including migration and tests.
    - Evidence: `CliLoginSession` stores only SHA-256 hashes of the browser
      nonce and poll secret; consume conditionally claims the session, revokes
      prior `d1v-cli` keys for the user/device, and creates a standard hashed
      API key. `20260830_cli_login` provides the schema migration; focused
      pytest covers pending, invalid secret, single consume, and rotation.
  - [x] Add browser login session client flow, private persistent device ID, and
    non-leaking CLI command surface.
    - Evidence: `d1v auth login --browser` appears in help and conflicts with
      other credential flags. It persists a random `~/.d1v/device-id` through
      a 0600 temp file + rename, opens (or prints) the nonce-only browser URL,
      polls via a secret header, then uses `TokenChain::save` for the API key.
  - [x] Complete the web approval handoff after every login method and expose
    device-key origin in API-key settings.
    - Evidence: email, password, OAuth, SUI, and Web3 completion paths approve
      CLI sessions after browser JWT issuance; OAuth retains the full callback
      URL. API key DTO/settings display the `d1v-cli` origin and retain normal
      revoke controls.
  - [x] Run targeted backend, CLI, and web validation; record evidence.
    - Evidence: `.venv/bin/python -m pytest -q backend_admin/tests/test_cli_login.py`
      (2 passed), `cargo fmt --all -- --check`, `cargo test --workspace`
      (160 CLI + 54 API unit + 14 client + 87 project + 32 user tests), CLI
      help/conflict smoke, `pnpm exec biome check` on changed Web files, and
      whitespace checks passed.

### Validator Handoff

- Result: passed for implementation and targeted validation.
- Checked: session authentication/expiry state, nonce and secret hashing,
  single consumption, device rotation, API-key persistence, browser command
  help/conflicts, changed web formatting, and full Rust workspace tests.
- Passed: targeted backend test suite, Rust formatter/test suite, CLI help and
  conflict smoke, and Biome for all changed frontend files.
- Failed: full `pnpm exec tsc --noEmit` remains blocked by pre-existing
  `dashboard-comps.tsx` `CircleDot` and dashboard translation-key errors.
- Not checked: a real browser-to-production endpoint flow, which needs a
  deployed backend and signed-in browser session.
- Risk: concurrent consumption has a conditional database update and row lock;
  production databases enforce both, while SQLite test locking is advisory.
- Plan update: complete for repository-scoped implementation; no deployment,
  migration application, commit, or push was requested.

## Commit Follow-up

- Goal: commit the browser device-login feature and use the available ARM64
  self-hosted runner for ordinary CLI CI execution.
- Todo:
  - [x] Move regular CLI CI Linux jobs to
    `[self-hosted, linux, arm64, d1v-linux-runner-arm64]`.
    - Evidence: push/PR `build` and manual/scheduled Linux `build-all` matrix
      entries now request the named runner; release and publishing workflows
      remain unchanged because they require hosted platform-specific runners.
  - [x] Commit the backend, CLI, frontend, migration, tests, docs, and runner
    changes after final status verification.
    - Evidence: CLI commit `c85b0ad` contains the browser flow and ARM64 CI
      runner configuration; parent and web commits record their respective
      backend/migration and approval-handoff changes.

# Goal: Automatically install the d1v Skill for detected coding agents

## Current Execution

- Goal: make ordinary d1v CLI installation gracefully install the official
  Skill only for Codex and Claude Code executables already available on PATH.
- Background: the installer currently requires an explicit target; `all`
  writes both destinations and can overwrite a user-modified Skill without a
  recoverable copy.
- Selected validators: `@cli-ux-qa`, `@cli-json-qa`, `@docs-adoption-qa`.
- Todo:
  - [x] Add `auto` detection, stable install reporting, and safe Skill backup
    behavior in the CLI while retaining explicit target semantics.
    - Evidence: auto resolves only executable `codex`/`claude` commands on
      PATH; explicit targets remain unconditional; different skills are
      timestamp-backed-up before an atomic replacement; JSON emits one result
      object.
  - [x] Make both release installers and the install page use `auto` by
    default; retain opt-out and explicit selection.
    - Evidence: both scripts pass `--agent auto` by default, perform
      `command -v` preflight, skip cleanly when neither executable exists, and
      accept all explicit modes; the install page defaults to Auto-detect and
      retains Codex, Claude, Both, and CLI-only selections.
  - [x] Add focused Rust/shell/web contract coverage, documentation, and run
    the required formatter, test, help, and JSON validation.
    - Evidence: `cargo fmt --all -- --check`, `cargo test --workspace`, skill
      and root help, JSON debug, isolated no-agent JSON smoke, installer E2E,
      shell syntax checks, and `pnpm test:skill` passed.

### Validator Handoff

- Result: passed for auto Skill installation and existing-skill preservation.
- Checked: PATH-only detection, explicit target behavior, no-agent success,
  JSON stdout, backup/update behavior, installer defaults, web command
  generation, docs, formatter, Rust workspace suite, and installer E2E.
- Passed: 158 CLI unit tests (10 Skill tests), 54 API unit tests, 14 API client tests, 87
  project tests, 32 user tests, shell checks, and both website skill contracts.
- Failed: `pnpm typecheck` remains blocked by pre-existing
  `dashboard-comps.tsx` `CircleDot`/translation-key errors and corresponding
  locale typing errors, outside this install route.
- Not checked: a live GitHub Release installer invocation; the script behavior
  is covered with the existing checksum/download mock.
- Risk: a process interruption after the old Skill has moved to its explicit
  backup but before the replacement rename can leave the backup in place for
  recovery; a normal replacement failure restores it automatically.
- Plan update: complete for repository-scoped implementation; no commit or
  push was requested in this turn.

# Goal: Preserve authentication across CLI upgrades

## Current Execution

- Goal: ensure `d1v upgrade` replaces only the executable and keeps the
  existing keyring/config authentication state available afterward.
- Background: users should not need to log in again after a binary upgrade.
- Selected validators: `@auth-state-qa`, `@cli-ux-qa`, `@docs-adoption-qa`.
- Todo:
  - [x] Lock the binary-only replacement boundary with a regression test and
    document the persistent auth stores.
    - Evidence: `install_binary` does not access auth state; the regression
      test confirms a config containing an API key remains byte-for-byte intact
      while the executable is replaced. Keyring identifiers remain the stable
      `d1v-cli`/`token` pair.
  - [x] Run focused and workspace validation.
    - Evidence: `cargo fmt --all -- --check` and `cargo test --workspace`
      passed after the auth-continuity test; installer and website contracts
      remain green.

### Validator Handoff

- Result: passed for authentication continuity across upgrades.
- Checked: executable replacement isolation, config preservation, stable
  keyring identifiers, upgrade documentation, and full Rust tests.
- Passed: 159 CLI tests including the new continuity test, plus all API tests.
- Failed: none for this change.
- Not checked: an interactive macOS Keychain upgrade, because it requires a
  live installed release and user keychain session.
- Risk: external keychain availability can still affect login lookup, but an
  upgrade itself does not mutate keyring or config state.
- Plan update: complete for repository-scoped implementation.

# Goal: Publish the canonical d1v coding-agent Skill from the CLI repository

## Current Execution

- Goal: make the existing official d1v Skill a versioned, reviewable asset in
  `d1v-cli`, retain the existing `d1v skill install` and website URLs, and
  update its deployment safety guidance for the current CLI release flow.
- Background: the installed Skill is currently served from a TypeScript string
  in the website repository. This prevents a directory PR from linking to a
  canonical `SKILL.md` in the official CLI repository and risks content drift.
- Selected validators: `@cli-ux-qa`, `@deploy-release-qa`,
  `@docs-adoption-qa`.
- Todo:
  - [x] Add the canonical `skills/d1v/SKILL.md` with verified CLI commands and
    deployment/release safety rules.
    - Evidence: the public Skill keeps the existing workspace and container
      guidance, removes unsafe secret-export defaults, and documents READY-only
      preview success plus interactive, explicit production confirmation.
  - [x] Point `d1v skill install` and the legacy website endpoint at that
    canonical file without changing their public entry points.
    - Evidence: `d1v skill install` defaults to the raw GitHub `SKILL.md`; the
      existing `https://www.d1v.ai/cli-skill.md` endpoint redirects there.
  - [x] Add content contract checks and run Rust, website, and command-surface
    validation; record the evidence below.
    - Evidence: `cargo fmt --all`, `cargo test --workspace`, `d1v skill install
      --help`, `d1v --help`, `d1v --format json debug`, and `pnpm test:skill`
      passed. `pnpm typecheck` is blocked by pre-existing dashboard icon and
      i18n translation-key errors outside the Skill route.

### Validator Handoff

- Result: passed for the canonical Skill, CLI installer source, legacy endpoint
  redirect, and documentation changes.
- Checked: SKILL frontmatter/production safeguards, CLI default URL and help,
  full Rust suite, JSON debug output, website redirect contract, README links.
- Passed: formatting; 54 API unit, 14 API client, 87 project, 32 user, and 151
  CLI tests; help/JSON smoke; `pnpm test:skill`; diff whitespace checks.
- Failed: none attributable to this change.
- Not checked: deployed website redirect. The pushed raw GitHub URL was fetched
  successfully and contains the expected frontmatter and deployment safeguards.
  The `d1vai` deploy workflow for commit `852e7ef` failed before assigning a
  runner or producing logs, matching the preceding website deploy failure.
- Risk: the website-wide `pnpm typecheck` currently fails on unrelated
  `dashboard-comps.tsx` and locale translation-key errors. The Skill route's
  dedicated contract test passes.
- Plan update: CLI source is published in `c5b279b`; the legacy endpoint source
  is published in `d1vai` commit `852e7ef` and awaits repair of the separate
  website deploy infrastructure before its live redirect can be confirmed.

# Goal: Add CLI request progress, confirmed deployments, and Homebrew tap automation

## Current Execution

- Goal: preserve existing install commands and CLI surface while adding terminal-only request spinners, confirmed Preview deployment polling, and a production Release API path when supported by the backend.
- Background: deployment shortcuts currently call the legacy production endpoint and Preview waits only 90 seconds; the API client has no unified request progress hook.
- Selected validators: `@cli-ux-qa`, `@cli-json-qa`, `@api-backend-qa`, `@deploy-release-qa`, `@docs-adoption-qa`.
- Todo:
  - [x] Add request progress callback and TTY-safe spinner lifecycle.
    - Evidence: `ProgressEvent`/`progress_handler` are wired through `d1v-api`; CLI enables an stderr spinner only for text output on a TTY.
  - [x] Share Preview polling and final URL output across shortcut and explicit commands.
    - Evidence: `wait_for_preview` is used by both paths, polls every 2 seconds for up to 10 minutes, handles terminal failure states, and prefers backend URLs.
  - [x] Add production Release API types/flow with confirmation and non-interactive refusal.
    - Evidence: preflight, environment decision prompts, idempotent release creation, status polling, success URL and detailed failure handling are implemented.
  - [x] Add Homebrew tap Formula/update workflow without changing install.sh.
    - Evidence: `Formula/d/d1v.rb` targets both macOS assets; `update-homebrew-tap.yml` verifies checksums and opens a PR using `HOMEBREW_TAP_TOKEN`.
  - [x] Run formatting, tests, help, and JSON output checks; record evidence and residual risk.
    - Evidence: `cargo fmt --all`, `cargo check -p d1v-cli`, all d1v-api/d1v-cli tests, `d1v --help`, and parseable `--format json debug` passed.

### Validator Handoff

- Result: passed for the implemented CLI/API and release automation changes.
- Checked: request callback lifecycle, TTY gating, Preview/production command routing, release types, Formula assets, workflow checksum guards, help and JSON output.
- Passed: formatting, compile, 54 d1v-api unit + 14 client + 87 project + 32 user + 147 CLI tests, help, and JSON smoke.
- Failed: none.
- Not checked: live backend Release endpoint contract, interactive terminal screenshots, GitHub repository creation, and macOS Homebrew CI because they require external services/credentials.
- Risk: Release endpoint paths and response semantics must match the deployed backend; the tap workflow intentionally requires a repository secret and a pre-created public `d1vai/homebrew-tap` repository. Spinner display is callback-based and does not suspend around every custom prompt yet.
- Plan update: complete for repository-scoped implementation; external Homebrew Core submission remains a follow-up after tap validation.

## Current Execution

- Goal: keep project selection usable when a user has a long project list, then release the completed CLI/API deployment changes.
- Background: the inline selector sized its terminal viewport to every option, so scrolling down could move the active project below the visible terminal area.
- Selected validators: `@cli-ux-qa`, `@cli-json-qa`, `@deploy-release-qa`.
- Todo:
  - [x] Bound project selector rendering and keep the active item visible.
    - Evidence: selector renders at most 12 options and follows the selected item; 3 unit tests cover long-list windowing and navigation to the last item.
  - [x] Verify workspace and release CLI/API version `0.1.27`.
    - Evidence: `cargo fmt --all`, `cargo test --workspace` (54 API unit + 14 client + 87 project + 32 user + 150 CLI tests), help, and valid JSON debug output passed.
  - [x] Commit, push, and tag `v0.1.27` to trigger release workflows.
    - Evidence: commit `3e46ccd` and tag `v0.1.27` are pushed; Publish and Release workflows passed, GitHub Release contains 7 target assets plus checksums, and crates.io returns HTTP 200 for `d1v-api 0.1.10` and `d1v-cli 0.1.27`.

## Current Execution

- Goal: publish the v0.1.27 Tap update and submit the source-build Formula to Homebrew core.
- Todo:
  - [x] Create a Tap PR with verified v0.1.27 asset checksums.
    - Evidence: https://github.com/d1vai/homebrew-tap/pull/1
  - [x] Submit a Homebrew core Formula PR using `cargo install --locked`.
    - Evidence: https://github.com/Homebrew/homebrew-core/pull/301328; local equivalent source build installed `d1v 0.1.27`.
  - [x] Repair the Tap workflow to use the Tap's Formula path and update script.
    - Evidence: workflow now updates `Formula/d1v.rb`, runs style/install/test, and reports a missing cross-repository token explicitly.

## Current Execution

- Goal: add `D1V_PROJECT_ID` to every production project, make the three highest-impact CLI interactions clearer with minimal changes, publish the CLI, update the local binary, and run the end-to-end command checks.
- Background: project creation now initializes the system variable for new projects, but existing production rows need an idempotent backfill. The current CLI has current-directory shortcut deployment code that needs discoverability and safer status behavior before release.
- Selected validators: `@project-lifecycle-qa`, `@deploy-release-qa`, `@cli-ux-qa`, `@cli-json-qa`, `@api-backend-qa`, `@docs-adoption-qa`.
- Todo:
  - [x] Backfill all production project environment rows and verify counts.
    - Evidence: production backfill created 1362 rows; a second run created 0 and found 1362 existing rows.
  - [x] Improve the top three CLI UX issues with focused tests and docs.
    - Evidence: current-directory shortcuts support existing-project selection or creation, atomic `.env` cloud merge with local-conflict preservation, and documented `--preview`/`--prev`/`--prod` usage.
  - [x] Publish a new d1v-cli version and update the local installation.
    - Evidence: CLI version bumped to 0.1.25 and pushed to `main`; local `/Users/apple/.local/bin/d1v` reports 0.1.25.
  - [x] Run full CLI/API flow and record residual risks.
    - Evidence: Rust 1.95 workspace check and 147 CLI unit tests pass; production deployment is healthy; unauthenticated API probe returns expected 403.

## Current Execution

- Goal: install the pinned enhanced Bash editor shipped in an Opcode runtime
  release next to the runtime binary and managed shell init.
- Background: runtime archives now contain `ble.sh/` in addition to
  `opcode-api` and `d1v-shell-init.sh`. The current CLI installer extracts the
  directory but only installs the binary and one shell-init file, so a device
  runtime upgrade would silently omit the editor required for history
  autosuggestions and command-line syntax highlighting.
- Selected validators: `@session-runtime-qa`, `@ops-reliability-qa`,
  `@security-privacy-qa`, `@migration-compat-qa`.
- Todo:
  - [x] Add a stable runtime editor asset path derived from the runtime binary.
    - Acceptance: a runtime installed at `<dir>/opcode-api` resolves the editor
      directory to `<dir>/ble.sh` without changing existing shell-init paths or
      command behavior.
    - Validators: `@session-runtime-qa`, `@migration-compat-qa`.
    - Evidence: `runtime_blesh_path` resolves an `opcode-api` sibling named
      `ble.sh`; the focused destination/install test passes without changing
      `runtime_shell_init_path`.
  - [x] Install the extracted editor directory using a staged replacement.
    - Acceptance: regular files and directories are copied, archive-provided
      symlinks and unsupported file types are rejected, a failed replacement
      leaves the previous usable directory recoverable, and stale staging paths
      are bounded to the exact runtime asset target.
    - Validators: `@ops-reliability-qa`, `@security-privacy-qa`.
    - Evidence: recursive installation uses exact `.new` and `.old` siblings,
      rejects links and special files, cleans failed staging, and restores the
      prior directory if activation fails. First install, replacement, and Unix
      symlink rejection tests pass.
  - [x] Add focused installer tests and run the repository validation gate.
    - Acceptance: tests cover the destination, successful recursive install,
      replacement of an older asset, and rejection of symlinks; `cargo fmt`,
      focused tests, full `cargo test`, CLI help, and JSON debug pass.
    - Validators: `@session-runtime-qa`, `@ops-reliability-qa`,
      `@security-privacy-qa`, `@migration-compat-qa`.
    - Evidence: `cargo fmt --all`, 10 focused runtime installer tests, all 147
      library tests, CLI `--help`, and parseable `--format json debug` pass.

### Validator Handoff

- Result: passed for runtime enhanced-editor asset installation.
- Checked: destination derivation, optional legacy-archive compatibility,
  recursive regular-file copying, staged replacement, cleanup, link rejection,
  focused/full tests, help output, and JSON output.
- Passed: formatting, 10 focused tests, all 147 library tests, CLI help, and
  parseable JSON debug output.
- Failed: none.
- Not checked: downloading a newly published production runtime archive; the
  opcode runtime release is published in a later controlled-rollout step.
- Risk: an interruption after moving the prior directory but before activating
  the replacement can leave the explicitly named `.old` directory for manual or
  subsequent-run recovery; no unbounded or user-selected path is removed.
- Plan update: complete; the Shell activation step can rely on the editor being
  installed beside the managed shell init.

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
