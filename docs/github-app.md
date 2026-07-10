# smbCloud GitHub App

The smbCloud GitHub App connects a GitHub repository to an smbCloud project so
that pushes to the production branch deploy automatically — no `smb deploy`
needed. Deploy results are reported back to GitHub as commit statuses
(`context: smbcloud/deploy`).

## How it works

1. You install the app on your GitHub account or organization and grant it
   access to one or more repositories.
2. `smb github connect` links a repository (and a production branch) to your
   smbCloud project's deploy repo.
3. On every push, GitHub delivers a `push` webhook to
   `https://api.smbcloud.xyz/v1/github/webhook`. The server verifies the
   webhook signature, matches the repository, installation, and branch against
   a connection, then builds and deploys the project's apps.
4. The server records a deployment (visible via `smb project deployment`) and
   sets a commit status on the pushed SHA: `pending` while building, then
   `success` or `failure`.

The webhook receiver and build pipeline are server-side; this repository ships
the app manifest, the CLI commands, and the client-side API contract.

## CLI usage

All commands accept `--project-id <id>`; without it they use the current
project (`smb project use --id <id>`). The project must have a deploy repo —
run `smb deploy` once to initialize it.

```sh
smb github install            # open the app installation page, wait for it to complete
smb github connect            # pick an installation + repository interactively
smb github connect --repo <owner>/<repo> --branch main   # non-interactive, CI-friendly
smb github status             # show the current connection
smb github disconnect         # remove the connection (asks for confirmation)
```

`smb github install` opens the browser and polls the API until a new
installation appears. A GitHub App's setup URL is fixed app-wide and cannot
point at `localhost`, so a local callback server (as used for OAuth login) is
not possible here — polling is the intended flow. If the poll times out, just
finish the installation in the browser and run `smb github connect`.

## Registering the app (operator, one-time)

The app is registered from [`github-app/manifest.json`](../github-app/manifest.json)
using GitHub's [manifest flow](https://docs.github.com/en/apps/sharing-github-apps/registering-a-github-app-from-a-manifest):

1. Serve a small HTML form that POSTs a `manifest` field containing the JSON to
   `https://github.com/settings/apps/new?state=<random>` (or
   `https://github.com/organizations/<org>/settings/apps/new?state=<random>`
   for an org-owned app).
2. GitHub redirects to the manifest's `redirect_url` with a temporary `code`.
3. Exchange it within one hour: `POST https://api.github.com/app-manifests/<code>/conversions`.
   The response contains the app id, slug, webhook secret, private key, and
   OAuth credentials — store these in your server configuration.
4. If the granted slug differs from `smbcloud`, update `GH_APP_SLUG` in
   `crates/smbcloud-networking/src/constants.rs` (the debug build uses a
   separate `smbcloud-dev` app whose webhook targets a dev API).

### Permissions and events

| Manifest entry | Why |
| --- | --- |
| `contents: read` | Fetch the repository at the pushed commit to build it. |
| `metadata: read` | Mandatory baseline for every GitHub App; powers repository listing. |
| `statuses: write` | Report deploy state as a commit status (one POST per state change). The Checks API adds no value for v1; upgrading to `checks: write` later is additive. |
| `default_events: ["push"]` | The only event auto-deploy needs. `installation` and `installation_repositories` events are always delivered to App webhooks without subscription — that is how the server tracks installs. |

## API contract

Endpoints the CLI calls (all under the standard authenticated `v1/` API;
implemented server-side):

| Method | Path | Request | Response |
| --- | --- | --- | --- |
| GET | `v1/github/installations` | — | `[GithubInstallation]` — installations belonging to the authenticated user (correlated via their linked GitHub account) |
| GET | `v1/github/installations/{installation_id}/repositories` | — | `[GithubRepository]` |
| GET | `v1/deploy_repos/{deploy_repo_id}/github_connection` | — | `GithubConnectionStatus` — always `200`, `connected: false` when absent |
| POST | `v1/deploy_repos/{deploy_repo_id}/github_connection` | `GithubConnectionCreate` | `GithubConnection` — upsert; omitted `production_branch` defaults to the repository's default branch |
| DELETE | `v1/deploy_repos/{deploy_repo_id}/github_connection` | — | `null` (`200`, idempotent) |
| POST | `v1/github/webhook` | GitHub webhook payload | server-only; never called by the CLI |

Response bodies must use `200`/`201` (not `204`). The serde models live in
`crates/smbcloud-model/src/github.rs`; a connection may also arrive embedded in
a `DeployRepo` payload as `github_connection`.
