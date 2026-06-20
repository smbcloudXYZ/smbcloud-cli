# CI / non-interactive deploys

`smb` is interactive by default — it prompts for confirmations, project setup,
credentials, and so on. In CI that's a problem: there's no TTY to answer a
prompt, so the command would hang or fail with a cryptic terminal error.

Pass the global **`--ci`** flag (or set **`SMB_CI=1`**, or run under any provider
that sets the conventional **`CI`** env var) to put `smb` in **non-interactive
mode**:

- Confirmations resolve to their default instead of asking
  (e.g. `smb --ci logout` proceeds without a y/n prompt).
- Any command that genuinely needs interactive input — `login`, `init`,
  `signup`, `account forgot-password`, `project new/update/delete`, and picking a
  monorepo target — **fails fast with a clear, actionable message** instead of
  blocking.

```sh
smb --ci deploy            # flag
SMB_CI=1 smb deploy        # env var
CI=true smb deploy         # most CI providers set this automatically
```

## Authenticate ahead of time

Logging in is interactive, so it can't run under `--ci`. Provision the token
once and make it available to the CI job at `~/.smb/token` (production) or
`~/.smb-dev/token` (dev). Typically you store the token contents in a CI secret
and write the file at the start of the job:

```yaml
- run: |
    mkdir -p ~/.smb
    printf '%s' "${{ secrets.SMB_TOKEN }}" > ~/.smb/token
    chmod 600 ~/.smb/token
```

Deploys also rsync over SSH using `~/.ssh/id_<user-id>@smbcloud`; install that
key the same way if your project deploys to a server tier.

## Example: the AircraftsHub monorepo

[AircraftsHub](https://aircraftshub.5mb.app) is a Tauri app whose Next.js web app
lives in the same repo (`web/`) as a pnpm workspace. Only the web app deploys to
smbCloud; the desktop/mobile app ships to the app stores. Its `.smb/config.toml`:

```toml
name = "aircraftshubweb"

[project]
id = 61
kind = "nextjs-ssr"
source = "web"                    # local Next.js app directory (workspace member)
path = "apps/web/aircraftshubweb" # remote runtime directory on the server
package_manager = "pnpm"
pm2_app = "aircraftshub-web"
port = 3022
```

Because the config pins the project, the source, and the runtime, deploying needs
no interaction at all:

```sh
smb --ci deploy
# ✔ Build complete.
# App is running ✔
# ✔ Deployment complete.
```

For a monorepo config with several `[[projects]]` entries, name the target —
`--ci` mode won't prompt you to choose:

```sh
smb --ci deploy --project aircraftshubweb
```

## Behavior reference

| Command | `--ci` behavior |
|---|---|
| `deploy` (config pins project) | Runs fully non-interactively |
| `deploy` (monorepo, no `--project`) | Fails: pass `--project <name>` |
| `deploy` (not authenticated) | Fails: provision the token first |
| `logout` | Proceeds (confirmation defaults to yes) |
| `login`, `init`, `signup`, `account forgot-password` | Fails fast — interactive only |
| `project new`, `project update`, `project delete` | Fails fast — interactive only |
| `me`, `migrate`, `project list/show`, `mail` | Unaffected (no prompts) |
