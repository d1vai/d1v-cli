# 发布到 crates.io

[`.github/workflows/publish.yaml`](../.github/workflows/publish.yaml) 在推送 `v*.*.*` tag 时把 `d1v-api` 和 `d1v-cli` 发布到 [crates.io](https://crates.io)。

## 1. 首次发布

crates.io 当前不允许为尚未存在的 crate 配置 Trusted Publishing，所以**首次发布必须用 API token**。

1. 在 crates.io 打开 **Account Settings → API Tokens → New Token**：
   - **Scopes**：`publish-new`、`publish-update`
   - **Expiration**：建议短期，例如 7 天
2. 在 GitHub 仓库打开 **Settings → Secrets and variables → Actions → New repository secret**：
   - **Name**：`CARGO_REGISTRY_TOKEN`
   - **Value**：上一步生成的 token
3. 在两个 crate 的 manifest 中设置版本，然后提交、打 tag、推送。

workflow 会使用 `CARGO_REGISTRY_TOKEN`，运行 `scripts/publish.py`，脚本会先发布 `d1v-api`，再发布 `d1v-cli`。

## 2. 切换到 Trusted Publishing

两个 crate 都存在于 crates.io 后，把 GitHub 配置为 Trusted Publisher，后续发布就不再需要长期 token。

对每个 crate：

1. 打开 crate 设置页：
   - <https://crates.io/crates/d1v-api/settings>
   - <https://crates.io/crates/d1v-cli/settings>
2. 添加 GitHub Trusted Publisher：
   - **Repository owner**：`d1vai`
   - **Repository name**：`d1v-cli`
   - **Workflow filename**：`publish.yaml`
   - **Environment**：留空
3. 删除 GitHub 仓库中的 `CARGO_REGISTRY_TOKEN` secret。
4. 在 crates.io 撤销首次发布用的 API token。

之后 workflow 会通过 [`rust-lang/crates-io-auth-action`](https://github.com/rust-lang/crates-io-auth-action) 获取短期 token，并通过 `CARGO_REGISTRY_TOKEN` 传给 `scripts/publish.py`。

## 3. 本地 dry run

不上传，只验证 package：

```sh
python scripts/publish.py --dry-run
```

只发布单个 crate，把 crate 名作为位置参数传入：

```sh
python scripts/publish.py --dry-run d1v-api
```

脚本仍会检查 crates.io，并跳过已发布版本。对于未发布版本，会执行 `cargo publish -p <crate> --locked --dry-run`。

> [!NOTE]
> 若 `d1v-api` 也未发布，`d1v-cli` 的 dry-run 会因 crates.io 索引上找不到对应版本而失败，这是 dry-run 的固有限制。

## 4. 常见错误

- **OIDC 步骤 401 / "publisher not configured"**：crate 没有配置 Trusted Publisher，或字段不匹配。按第 2 节配置，或临时设置 `CARGO_REGISTRY_TOKEN`。
- **`d1v-cli` 报 "no matching package named d1v-api"**：crates.io 还没有索引刚发布的 `d1v-api`。稍等一分钟后重跑 workflow。
- **某个 crate 被意外跳过**：同一个 crate/version 已经存在于 crates.io。升级该 crate 的 `[package].version` 后重新打 tag。
