# Publishing to crates.io

[`.github/workflows/publish.yaml`](../.github/workflows/publish.yaml) publishes `d1v-api` and `d1v-cli` to [crates.io](https://crates.io) when a `v*.*.*` tag is pushed.

## 1. First publish

crates.io does not yet allow configuring Trusted Publishing for a crate that does not exist, so the **first publish must use an API token**.

1. On crates.io, open **Account Settings → API Tokens → New Token**:
   - **Scopes**: `publish-new`, `publish-update`
   - **Expiration**: short, for example 7 days
2. In the GitHub repo, open **Settings → Secrets and variables → Actions → New repository secret**:
   - **Name**: `CARGO_REGISTRY_TOKEN`
   - **Value**: the token from step 1
3. Set versions in both crate manifests, then commit, tag, and push.

The workflow uses `CARGO_REGISTRY_TOKEN`, runs `scripts/publish.py`, and the script publishes `d1v-api` before `d1v-cli`.

## 2. Switch to Trusted Publishing

Once both crates exist on crates.io, configure GitHub as a Trusted Publisher so future releases do not need a long-lived token.

For each crate:

1. Open the crate settings page:
   - <https://crates.io/crates/d1v-api/settings>
   - <https://crates.io/crates/d1v-cli/settings>
2. Add a GitHub Trusted Publisher:
   - **Repository owner**: `d1vai`
   - **Repository name**: `d1v-cli`
   - **Workflow filename**: `publish.yaml`
   - **Environment**: leave empty
3. Delete the `CARGO_REGISTRY_TOKEN` secret from the GitHub repo.
4. Revoke the API token used for the first publish on crates.io.

The workflow will then request a short-lived token through [`rust-lang/crates-io-auth-action`](https://github.com/rust-lang/crates-io-auth-action) and pass it to `scripts/publish.py` through `CARGO_REGISTRY_TOKEN`.

## 3. Local dry run

To test package validation without uploading:

```sh
python scripts/publish.py --dry-run
```

To target a single crate, pass its name as a positional argument:

```sh
python scripts/publish.py --dry-run d1v-api
```

The script still checks crates.io and skips already published versions. For unpublished versions, it runs `cargo publish -p <crate> --locked --dry-run`.

> [!NOTE]
> If `d1v-api` is also unpublished, the dry-run for `d1v-cli` will fail because the matching version is not on the crates.io index — an inherent limitation of dry-run.

## 4. Troubleshooting

- **OIDC step fails with 401 / "publisher not configured"** — Trusted Publisher is not configured for that crate, or the fields do not match. See section 2, or temporarily set `CARGO_REGISTRY_TOKEN`.
- **`d1v-cli` fails with "no matching package named d1v-api"** — crates.io has not indexed the newly published `d1v-api` yet. Re-run the workflow after a minute.
- **A crate was skipped unexpectedly** — the same crate/version already exists on crates.io. Bump the crate's `[package].version` and tag again.
