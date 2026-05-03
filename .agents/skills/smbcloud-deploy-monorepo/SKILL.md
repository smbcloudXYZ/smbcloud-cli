---
name: smbcloud-deploy-monorepo
description: Use when deploying from a monorepo with smbCloud CLI, especially repositories that contain multiple deployable sub-projects with different strategies (Next.js SSR, Vite SPA, Rails, static) behind a single `.smb/config.toml` with `runner = 255` and `[[projects]]` entries.
---

# smbCloud Deploy Monorepo

Use this skill when work touches monorepo deployment on smbCloud.

Applies to:

- repositories with `runner = 255` in `[project]`
- `.smb/config.toml` files with `[[projects]]` arrays
- `process_deploy.rs` monorepo resolution logic
- sub-project routing to `nextjs-ssr`, `vite-spa`, `rails`, rsync, or git deploy paths
- multi-tenant PM2 and Nginx server configuration
- CI workflows that deploy individual sub-projects from a monorepo

## What makes a monorepo

A monorepo is a repository where `[project]` has `runner = 255` (the `Monorepo` variant) and deployable sub-projects are listed as `[[projects]]` entries. The root project is never deployed directly — it is a container.

The CLI detects this in `process_deploy.rs`:

```rust
let resolved_name = match project_name {
    Some(name) => Some(name),
    None if config.project.runner == Runner::Monorepo => Some(prompt_select_project(&config)?),
    None => None,
};
```

When `runner = 255` and no `--project` flag is passed, the CLI shows an interactive selection prompt. When `--project <name>` is passed, the prompt is skipped.

After selection, `resolve_sub_project()` swaps `config.project` with the matching `[[projects]]` entry. From that point, all downstream deploy code operates on a single `Project` — it does not know or care that it came from a monorepo.

## Config structure

A monorepo `.smb/config.toml` has two layers:

### Root project (container)

```toml
name = "splitfire"
description = "SplitFire AI monorepo."

[project]
id = 38
name = "splitfire"
repository = "splitfire"
runner = 255
description = "SplitFire AI monorepo."
created_at = "2025-09-20T16:33:05.154Z"
updated_at = "2025-09-20T16:33:05.154Z"
```

Key rules:

- `runner = 255` is mandatory — this is what activates monorepo behavior
- `id` must be a valid smbCloud project ID (created via `smb init` or the web console)
- the root project has no `kind`, `source`, `path`, or `pm2_app` — it is not deployable

### Sub-projects (deployable units)

Each `[[projects]]` entry is a self-contained deploy target with its own `kind`, `source`, `path`, `runner`, and strategy-specific fields.

```toml
[[projects]]
id = 54
name = "splitfireweb"
repository = "splitfireweb"
description = "SplitFire AI website."
source = "frontend/splitfire-web"
path = "apps/web/splitfireweb"
runner = 0
kind = "nextjs-ssr"
package_manager = "pnpm"
pm2_app = "splitfire-web"
port = 3010
created_at = "2025-09-20T16:33:05.154Z"
updated_at = "2025-09-20T16:33:05.154Z"

[[projects]]
id = 98
name = "musik88web"
repository = "musik88-production"
description = "Musik88 web or SplitFire AI API."
source = "backend/musik88-web"
runner = 2
kind = "rails"
shared_lib = "lib"
compile_cmd = "cd ~/lib/gems/gem_error_codes && rbenv local 3.4.2 && bundle install && bundle exec rake compile"
created_at = "2025-09-20T16:33:05.154Z"
updated_at = "2025-09-20T16:33:05.154Z"

[[projects]]
id = 53
name = "connecteddevices"
repository = "connecteddevices"
description = "KaroKowe connected devices TV web app."
source = "frontend/karokowe-connected-devices"
path = "apps/web/connecteddeviceskarokowe"
runner = 0
kind = "vite-spa"
output = "dist"
package_manager = "pnpm"
created_at = "2025-09-20T16:33:05.154Z"
updated_at = "2025-09-20T16:33:05.154Z"
```

## Sub-project field reference

| Field               | Required | Description                                                                                 |
| ------------------- | -------- | ------------------------------------------------------------------------------------------- |
| `id`                | yes      | smbCloud project ID                                                                         |
| `name`              | yes      | unique identifier, used with `--project` flag and in the selection prompt                   |
| `repository`        | yes      | remote repository name on the smbCloud server (used for git deploy and SSH paths)           |
| `description`       | no       | human-readable description                                                                  |
| `source`            | yes      | local path to the sub-project directory, relative to the monorepo root                      |
| `path`              | depends  | remote directory on the server, relative to `~/` — required for `nextjs-ssr` and `vite-spa` |
| `runner`            | yes      | server tier: `0` (NodeJs), `1` (Static), `2` (Ruby), `3` (Swift)                            |
| `kind`              | depends  | deploy strategy: `"nextjs-ssr"`, `"vite-spa"`, `"rails"`, or omitted for generic deploy     |
| `package_manager`   | depends  | `"pnpm"` or `"npm"` — required for `nextjs-ssr` and `vite-spa`                              |
| `pm2_app`           | depends  | PM2 process name — required for `nextjs-ssr`                                                |
| `port`              | depends  | runtime port — required for `nextjs-ssr`, defaults to `3000` if omitted                     |
| `output`            | depends  | build output directory — required for `vite-spa`, typically `"dist"`                        |
| `shared_lib`        | no       | path to shared library directory to rsync before deploy — used by `rails`                   |
| `compile_cmd`       | no       | SSH command to run on the server after syncing shared libs — used by `rails`                |
| `deployment_method` | no       | `0` (Git) or `1` (Rsync) — only matters when `kind` is not set                              |
| `created_at`        | yes      | ISO 8601 timestamp                                                                          |
| `updated_at`        | yes      | ISO 8601 timestamp                                                                          |

## Deploy routing

The `kind` field drives strategy selection. This happens after `resolve_sub_project()` has already swapped the active project, so routing logic is identical for standalone and monorepo projects.

```
smb deploy --project <name>
         │
         ▼
   resolve_sub_project()
         │
         ▼
   config.project.kind?
         │
    ┌────┼──────────┬──────────────┐
    ▼    ▼          ▼              ▼
 vite-spa  nextjs-ssr  rails    (none)
    │       │          │           │
    ▼       ▼          ▼           ▼
 pnpm    pnpm       rsync    deployment_method?
 build   install     lib/         │
   │     + build    compile    ┌──┴──┐
   ▼       │        git push   Git  Rsync
 rsync     ▼          │        │     │
 dist/   rsync 3      ▼        ▼     ▼
         dirs       post-    git   rsync
           │       receive   push   -a
           ▼         hook
         SSH PM2
         restart
```

### Strategy: `nextjs-ssr`

Local build, rsync three directories, SSH PM2 restart. See `smbcloud-deploy-nextjs` skill for details.

Required fields: `kind`, `source`, `path`, `pm2_app`, `package_manager`, `port`

### Strategy: `vite-spa`

Local build, rsync the output directory. No PM2 — Nginx serves static files directly.

Required fields: `kind`, `source`, `path`, `output`, `package_manager`

### Strategy: `rails`

Rsync shared libraries, SSH compile native extensions, git force-push the sub-project directory to the server's bare repo (triggers post-receive hook).

Required fields: `kind`, `source`, `repository`, `runner = 2`

Optional: `shared_lib`, `compile_cmd`

### Strategy: generic git or rsync

When `kind` is omitted, the `deployment_method` field controls the path. Git push uses libgit2 to push to the server's bare repo. Rsync uses system `rsync` to sync the source tree.

## The `source` field

The `source` field is the most important monorepo-specific field. It tells the CLI where the sub-project lives relative to the monorepo root.

For `nextjs-ssr` and `vite-spa`, the CLI changes into the `source` directory before running build commands:

- `pnpm install` runs inside `source/`
- `pnpm build` runs inside `source/`
- rsync sources are relative to `source/`

For `rails`, the CLI:

- rsyncs `shared_lib` from the monorepo root (not from `source`)
- runs `compile_cmd` on the server via SSH
- initializes a temporary git repo inside `source/` and force-pushes it

The `source` path must be a valid directory relative to where `smb deploy` is run (the monorepo root).

## Runner types and server tiers

Each sub-project's `runner` determines which smbCloud server receives the deployment:

| Runner     | Value | SSH Host             | Use Case                                  |
| ---------- | ----- | -------------------- | ----------------------------------------- |
| `NodeJs`   | `0`   | `api.smbcloud.xyz`   | Next.js, Vite, Node.js apps               |
| `Static`   | `1`   | `api.smbcloud.xyz`   | Pure static sites (Nginx serves directly) |
| `Ruby`     | `2`   | `api-1.smbcloud.xyz` | Rails apps                                |
| `Swift`    | `3`   | `api-1.smbcloud.xyz` | Vapor apps                                |
| `Monorepo` | `255` | —                    | Container only, never deployed directly   |

Sub-projects within the same monorepo can target different runners. The SplitFire monorepo demonstrates this: `splitfireweb` (runner 0, api.smbcloud.xyz) and `musik88web` (runner 2, api-1.smbcloud.xyz) deploy to different servers from the same repository.

## Multi-tenant server layout

Multiple apps share the same server, each in its own directory under the deploy user's home:

```
/home/git/
├── apps/
│   └── web/
│       ├── splitfireweb/           # Next.js SSR (PM2: splitfire-web, port 3010)
│       │   ├── server.js
│       │   ├── .next/static/
│       │   └── public/
│       ├── connecteddeviceskarokowe/ # Vite SPA (no PM2, Nginx serves directly)
│       │   └── index.html
│       ├── ondeinference.com/       # Next.js SSR (PM2: ondeinference-web, port 3026)
│       │   ├── server.js
│       │   ├── .next/static/
│       │   └── public/
│       └── aircraftshubweb/         # Next.js SSR (PM2: aircraftshub-web, port 3022)
│           ├── server.js
│           ├── .next/static/
│           └── public/
└── musik88-production.git/          # Bare repo (Rails post-receive hook)
```

Each `nextjs-ssr` app runs as a separate PM2 process on a unique port. Nginx reverse-proxies each domain to the correct port. Vite SPA and static apps have no PM2 process — Nginx serves files directly with `try_files`.

### Port allocation

Every `nextjs-ssr` sub-project must have a unique `port` value. Ports are allocated manually and must be consistent across three places:

1. `.smb/config.toml` — the `port` field in the `[[projects]]` entry
2. PM2 — the `PORT` environment variable passed to `node server.js`
3. Nginx — the `proxy_pass` upstream address

If any of these disagree, the app will either not start, not be reachable, or serve the wrong app.

Current allocations on `api.smbcloud.xyz`:

| App               | PM2 Name            | Port |
| ----------------- | ------------------- | ---- |
| splitfire-web     | `splitfire-web`     | 3010 |
| aircraftshub-web  | `aircraftshub-web`  | 3022 |
| ondeinference-web | `ondeinference-web` | 3026 |

### Nginx configuration per app type

**Next.js SSR apps** use reverse proxy:

```nginx
upstream app_name {
    server 127.0.0.1:<port>;
    keepalive 64;
}

server {
    server_name example.com;

    location /_next/static/ {
        alias /home/git/apps/web/<app>/.next/static/;
        expires max;
        add_header Cache-Control "public, max-age=31536000, immutable";
        access_log off;
    }

    location / {
        proxy_pass http://app_name;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
        proxy_buffering off;
    }
}
```

**Vite SPA / static apps** use direct file serving:

```nginx
server {
    server_name example.com;
    root /home/git/apps/web/<app>/;

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

Do not mix these patterns. An SSR app with `try_files` will return 404 for every server-rendered route. A static app with `proxy_pass` will fail because there is no PM2 process to proxy to.

## CLI usage

### Interactive mode

```sh
cd my-monorepo/
smb deploy
# → "Select project to deploy"
# → 1. splitfireweb
#    2. musik88web
#    3. connecteddevices
```

### Direct mode (CI and scripts)

```sh
smb deploy --project splitfireweb
smb deploy -p musik88web
smb deploy -p connecteddevices
```

The `--project` / `-p` value must match the `name` field in a `[[projects]]` entry exactly. It is case-sensitive.

## CI workflow parity

GitHub Actions workflows mirror the CLI deploy strategies. Each sub-project has its own workflow file triggered by a dedicated deploy branch.

### Next.js SSR CI deploy

Trigger: push to `release/splitfire-web`

```yaml
env:
  GIT_HOST: api.smbcloud.xyz
  GIT_USER: git
  REMOTE_PATH: apps/web/splitfireweb
  PM2_APP: splitfire-web

steps:
  - name: Build
    run: |
      cd frontend/splitfire-web
      pnpm install --ignore-scripts
      pnpm build

  - name: Upload
    run: |
      cd frontend/splitfire-web
      rsync -az --delete --mkpath .next/standalone/ $GIT_USER@$GIT_HOST:$REMOTE_PATH/
      rsync -az --delete --mkpath .next/static/ $GIT_USER@$GIT_HOST:$REMOTE_PATH/.next/static/
      rsync -az --delete --mkpath public/ $GIT_USER@$GIT_HOST:$REMOTE_PATH/public/

  - name: Restart
    run: |
      ssh $GIT_USER@$GIT_HOST bash << 'EOF'
        set -e
        cd ~/apps/web/splitfireweb
        if pm2 describe "splitfire-web" > /dev/null 2>&1; then
          pm2 restart "splitfire-web" --update-env
        else
          PORT=3010 HOSTNAME=127.0.0.1 pm2 start node --name "splitfire-web" -- server.js
        fi
        pm2 save
      EOF
```

### Rails CI deploy

Trigger: push to `release/backend-musik88-web`

```yaml
steps:
  - name: Upload shared lib
    run: rsync -r ./lib $GIT_USER@$GIT_HOST:~/

  - name: Compile native gem
    run: ssh $GIT_USER@$GIT_HOST 'bash -s' < compile.sh ./lib/gems/gem_error_codes

  - name: Git force-push
    run: |
      cd backend/musik88-web
      git init
      git add .
      git commit -m "Deploy to production"
      git remote add prod $GIT_USER@$GIT_HOST:musik88-production
      git push --set-upstream prod main --force
```

### Vite SPA CI deploy

Trigger: push to the relevant deploy branch

```yaml
steps:
  - name: Build
    run: |
      cd frontend/karokowe-connected-devices
      pnpm install
      pnpm build

  - name: Upload
    run: |
      cd frontend/karokowe-connected-devices
      rsync -az --delete --mkpath dist/ $GIT_USER@$GIT_HOST:apps/web/connecteddeviceskarokowe/
```

### CI rsync flags

All CI rsync commands must include:

- `--mkpath` — creates remote directories if they do not exist (prevents failure on first deploy)
- `--delete` — removes stale files on the remote (prevents serving old assets)
- `-az` — archive mode with compression

### CI PM2 restart

The CI restart command must use `pm2 restart <app> --update-env` on subsequent deploys so environment variable changes from the ecosystem file are picked up. Without `--update-env`, PM2 keeps the environment from the original `pm2 start` invocation.

## Deployment tracking

Every deploy records status in the smbCloud API regardless of strategy:

1. `POST /deployments` with `status: Started` before transferring files
2. `PATCH /deployments/:id` with `status: Done` on success
3. `PATCH /deployments/:id` with `status: Failed` on failure

For non-git deploys (rsync, nextjs-ssr, vite-spa), the `commit_hash` field is a UTC timestamp (`20250920T163305Z`) since there is no git commit on the deploy path.

## Adding a new sub-project to a monorepo

1. Create the project in smbCloud (via `smb init` in a temporary directory, or the web console) to get an `id`
2. Add a `[[projects]]` entry to the monorepo's `.smb/config.toml` with the correct `kind`, `source`, `path`, `runner`, and strategy-specific fields
3. If `kind = "nextjs-ssr"`: allocate a unique port, configure PM2 ecosystem file on the server, add Nginx reverse proxy config
4. If `kind = "vite-spa"`: add Nginx static file config
5. If `kind = "rails"`: set up a bare git repo on the server with a post-receive hook
6. Test with `smb deploy -p <name>`

## Removing a sub-project

1. Remove the `[[projects]]` entry from `.smb/config.toml`
2. On the server: `pm2 delete <pm2_app> && pm2 save` (for SSR apps)
3. On the server: remove the Nginx config and `sudo nginx -t && sudo systemctl reload nginx`
4. On the server: remove the app directory

## Standalone repos vs monorepo sub-projects

A standalone repo (like `aircraftshub-web` or `ondeinference.com`) has its own `.smb/config.toml` with `kind` and deploy fields directly in `[project]`. It has no `[[projects]]` array and `runner` is not `255`.

A monorepo sub-project has the same fields, but they live inside a `[[projects]]` entry. After `resolve_sub_project()`, the CLI treats them identically — all strategy code operates on `config.project` regardless of origin.

To convert a standalone repo into a monorepo sub-project:

1. Move the `[project]` fields into a `[[projects]]` entry
2. Create a new `[project]` with `runner = 255`
3. Set `source` to the sub-directory path
4. Adjust `path` if the remote directory structure differs

## Debugging monorepo deploys

### "No [[projects]] entries found"

The root `[project]` has `runner = 255` but there are no `[[projects]]` entries. Add at least one sub-project.

### "Sub-project 'foo' not found in [[projects]]"

The `--project` name does not match any `[[projects]]` entry's `name` field. Check spelling and case.

### Wrong sub-project deploys

If the interactive prompt selects the wrong project, verify the `[[projects]]` order in `.smb/config.toml` — the prompt lists them in file order.

### Build runs in wrong directory

The `source` field is incorrect. It must be relative to the monorepo root where `smb deploy` is run.

### Port conflict on multi-tenant server

Two `nextjs-ssr` apps have the same `port` value. PM2 will start both, but only one will bind the port. Check `pm2 list` and compare ports across all `.smb/config.toml` files that target the same server.

### Stale assets after deploy

The Nginx `alias` directive for `/_next/static/` or the `root` for a Vite SPA points to the wrong directory or a stale deploy path. Verify the Nginx config matches the `path` field in `.smb/config.toml`.

### PM2 not picking up new environment variables

Use `pm2 restart <app> --update-env && pm2 save`. Without `--update-env`, PM2 reuses the environment from the original start.

## Validation

### Config

- every `[[projects]]` entry has a unique `name`
- every `[[projects]]` entry has a unique `id`
- every `nextjs-ssr` entry has a unique `port`
- `source` directories exist on disk
- `runner` values match the intended server tier

### Local

- `smb deploy -p <name>` completes without error for each sub-project
- `cargo check -p smbcloud-cli` passes after any deploy code changes

### Server

- `pm2 list` shows expected processes with correct names and ports
- `sudo nginx -t` passes
- each domain resolves to the correct app content
- `ls -la /home/git/apps/web/<app>/` confirms expected files

## Common mistakes

- setting `runner = 255` on a standalone project that has no `[[projects]]` entries
- forgetting to set `runner = 255` on the root `[project]` of a monorepo — the CLI treats it as a standalone project and tries to deploy the root
- duplicating `port` values across sub-projects on the same server
- using `kind = "nextjs-ssr"` on a project without `output: "standalone"` in its Next.js config
- running `smb deploy` from a subdirectory instead of the monorepo root — `source` paths will not resolve
- forgetting `--mkpath` on rsync in CI workflows — first deploys fail because the remote directory does not exist
- keeping stale Nginx configs pointing at old deploy paths after migrating to a new directory structure
- mixing up `path` (remote destination) and `source` (local directory) — they serve different purposes
- using `pm2 restart` without `--update-env` after changing ecosystem file variables
- assuming the CLI generates Nginx configs — it does not; Nginx is always configured manually on the server
