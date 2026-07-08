---
name: smbcloud-deploy-nextjs
description: Use when deploying or debugging Next.js apps on smbCloud, especially the dedicated `nextjs-ssr` flow that builds locally, uploads `.next/standalone` via rsync, restarts PM2 over SSH, and serves traffic behind Nginx.
---

# smbCloud Deploy Next.js

Use this skill when work touches Next.js deployment on smbCloud.

Applies to:

- `kind = "nextjs-ssr"` deploy targets
- `.smb/config.toml` deploy configuration
- `process_deploy_nextjs_ssr.rs`
- PM2 process management for standalone builds
- Nginx proxying for SSR apps
- production-only issues caused by CORS, ports, or runtime mismatch

## Deployment model

Keep the deploy hierarchy straight:

- `Project` = umbrella workspace in smbCloud (not deployable)
- `DeployRepo` = the git repository inside that workspace
- `FrontendApp` = the deployable app unit, identified by `frontend_app_id` in config
- `Deployment` = one release event, linked to a FrontendApp via `frontend_app_id`

The canonical chain is: Deployment → FrontendApp → DeployRepo → Project

For `nextjs-ssr`, smbCloud does not use git-push deploy.

The CLI route in `process_deploy.rs` short-circuits on:

- `kind = "nextjs-ssr"`

That path builds locally, then:

- rsyncs `.next/standalone/` into the remote app directory root
- rsyncs `.next/static/` into the runtime directory's `.next/static/`
- rsyncs `public/` into the runtime directory's `public/`
- runs `ssh` and restarts the app with PM2

When `outputFileTracingRoot` points above the app directory, Next may preserve the source path inside `.next/standalone/` (for example `.next/standalone/web/server.js`). The deploy path must keep that nested runtime layout intact.

Do not describe this flow as git based unless the project is actually using the generic git deploy path.

## Required app configuration

The Next.js app must produce a standalone server bundle.

Create `next.config.mjs` or equivalent with:

```js
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
};

export default nextConfig;
```

Without `output: "standalone"`, the deploy code fails because `.next/standalone/` is missing.

The app package scripts should support:

- `build`: `next build`

`pnpm` is the safe package manager for the current CLI implementation.

## Required smbCloud config

Minimum `.smb/config.toml` for SSR:

```toml
[project]
id = 28                    # workspace id (for API routing)
frontend_app_id = "uuid"   # FrontendApp id (for deployment tracking)
deploy_repo_id = 123       # DeployRepo id (links to git repository record)
deployment_method = 1
kind = "nextjs-ssr"
source = "."
package_manager = "pnpm"
pm2_app = "my-app"
path = "apps/web/my-app"
port = 3028
```

Important rules:

- `kind = "nextjs-ssr"` is what activates the SSR deploy path
- `id` is the umbrella workspace ID, used for auth and API routing
- `frontend_app_id` identifies the FrontendApp record for this deploy target — include it so deployments are attributed to the correct app, not just the workspace
- `deploy_repo_id` identifies the DeployRepo record backing this app — include it when available
- `source` is the local Next.js app directory
- `path` is the remote destination directory on the server
- `pm2_app` is mandatory because the CLI restarts PM2 by name
- `port` should be set explicitly and must match nginx
- `deployment_method = 1` may still be present, but the `kind` routing is what matters most

## Expected remote layout

After a successful deploy, the remote directory should look like one of these layouts:

- flat runtime
  - `/home/git/apps/web/<app>/server.js`
  - `/home/git/apps/web/<app>/.next/static/...`
  - `/home/git/apps/web/<app>/public/...`
- nested runtime when standalone preserves the source path
  - `/home/git/apps/web/<app>/node_modules/...`
  - `/home/git/apps/web/<app>/web/server.js`
  - `/home/git/apps/web/<app>/web/.next/static/...`
  - `/home/git/apps/web/<app>/web/public/...`

The CLI uploads the contents of `.next/standalone/`, not the folder itself. In nested-runtime mode it also needs to keep PM2 compatibility for old root-level `server.js` and `.next/standalone/server.js` entrypoints.

## PM2 behavior

The CLI's deploy path now deletes and starts fresh, preferring the server's
ecosystem config file when it exists. The CLI checks for `ecosystem.config.cjs`
first, then falls back to `ecosystem.config.js`. During deploy, the server-side
migration auto-renames `.js` to `.cjs` when applicable.

Conceptually it behaves like:

```sh
cd "$APP_PATH"
mkdir -p logs

if pm2 describe "$PM2_APP" > /dev/null 2>&1; then
  pm2 delete "$PM2_APP"
fi

if [ -f ecosystem.config.cjs ]; then
  pm2 start ecosystem.config.cjs --only "$PM2_APP" --env production
elif [ -f ecosystem.config.js ]; then
  pm2 start ecosystem.config.js --only "$PM2_APP" --env production
else
  NODE_ENV=production PORT=<port> HOSTNAME=127.0.0.1 pm2 start node --name "$PM2_APP" -- server.js
fi

pm2 save
```

This has two consequences:

- old git-based `post-receive` hooks are irrelevant for the SSR path
- the configured `port` in `.smb/config.toml` is used for the fallback start path
- the live server `ecosystem.config.cjs` (or `.js`) is the runtime source of truth when present

### Environment variables — ecosystem file (standard pattern)

The server runs in a **multi-tenant setup**: multiple Next.js apps share one server, each managed as a separate PM2 process. The standard way to manage per-app environment variables is an `ecosystem.config.cjs` file kept **on the server only** (never committed to the app repo).

Use the `.cjs` extension. This is required when the project's `package.json` has `"type": "module"`, because Node treats `.js` files as ESM in that case and `module.exports` syntax will fail.

The flow works like this:

1. On first setup, SSH in, create the ecosystem file, start with it, and save:

   ```sh
   ssh git@<server>
   cd /home/git/apps/web/<app>

   cat > ecosystem.config.cjs << 'EOF'
   module.exports = {
     apps: [{
       name: "<pm2_app>",
       script: "server.js",
       cwd: "/home/git/apps/web/<app>",
       env_production: {
         NODE_ENV:  "production",
         PORT:      3026,
         HOSTNAME:  "127.0.0.1",
         MY_SECRET: "...",
       }
     }]
   }
   EOF

   pm2 start ecosystem.config.cjs --env production
   pm2 save
   ```

2. Every subsequent CLI deploy deletes the PM2 process and starts it fresh. The CLI checks for `ecosystem.config.cjs` first, then `ecosystem.config.js`. If neither exists, it falls back to `node server.js` with inline `NODE_ENV`, `PORT`, and `HOSTNAME`.

3. To change or add env vars, SSH in, edit `ecosystem.config.cjs`, then:

   ```sh
   pm2 restart <pm2_app> --update-env
   pm2 save
   ```

The ecosystem file is the server-side source of truth for each app's runtime config. The CLI preserves both `ecosystem.config.cjs` and `ecosystem.config.js` on the server during rsync and must not delete either.

### Manual PM2 operations (without ecosystem file)

If you are operating without an ecosystem file, prefer the same fresh-start model the CLI uses:

```sh
cd /home/git/apps/web/<app>
if pm2 describe <pm2_app> > /dev/null 2>&1; then
  pm2 delete <pm2_app>
fi
PORT=<port> HOSTNAME=127.0.0.1 pm2 start node --name <pm2_app> -- server.js
pm2 save
```

If you are using an ecosystem file and only changed env vars manually, this is still valid:

```sh
cd /home/git/apps/web/<app>
pm2 restart <pm2_app> --update-env
pm2 save
```

## Port management

Use one source of truth for the runtime port.

If an older ecosystem config (`.cjs` or `.js`) exists and contains both:

- `args: "start -p 3026"`
- `env_production.PORT = 3025`

that is a broken configuration. Standardize on one port value.

Current rules:

- fallback fresh-start path uses `port` from `.smb/config.toml`
- ecosystem-managed apps should set the same port in `env_production.PORT` (in `ecosystem.config.cjs`)
- nginx upstream must match that port

## Nginx for SSR

Do not use a static-site Nginx config like:

```nginx
root /home/git/apps/web/my-app;
location / {
    try_files $uri $uri/ =404;
}
```

That is for static files, not Next.js SSR.

Use reverse proxying to the local Node process instead:

```nginx
location / {
    proxy_pass http://127.0.0.1:3026;
    proxy_http_version 1.1;

    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;

    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";

    proxy_cache_bypass $http_upgrade;
}
```

Nginx must proxy to the actual PM2 port.

### Do not `alias` `/_next/static` for nested standalone builds

The proxy-everything block above is the safe default: let the standalone Node
server serve `/_next/static` itself. The common breakage is an nginx block that
tries to serve those assets from disk:

```nginx
# WRONG for a nested (monorepo) standalone build
location /_next/static {
    alias /home/git/apps/web/<app>/.next/static;   # 404s every JS/CSS
}
```

When the app builds with `outputFileTracingRoot` (monorepo), standalone is
**nested**, so static actually lands at `…/<app>/<source>/.next/static`
(e.g. `…/<app>/apps/web/.next/static`), not `…/<app>/.next/static`. The alias
points at the wrong directory and every chunk 404s while the HTML still loads —
a confusing "page renders, no styles/JS" symptom.

Fixes, in order of preference:

1. Drop the `/_next/static` `alias` block and proxy everything to the Node
   process (the block above). Layout-independent — works for flat and nested.
2. If you must serve static from disk, point the alias at the nested path:
   `alias …/<app>/<source>/.next/static;`.

Flat (single-repo, `source = "."`) builds keep static at `…/<app>/.next/static`,
so a disk alias there happens to be correct — but proxy-everything still works
and is the recommended uniform pattern.

### One file + SAN cert for multiple hostnames

To serve several hostnames for the same app (e.g. a primary domain plus a
`*.5mb.app` alias) from one `server` block, they must share one certificate — a
`server` block can only present one cert. Issue a SAN cert listing both names,
then write one block:

```sh
sudo certbot certonly --nginx -d app.example.com -d app.example.net --cert-name myapp
```

```nginx
server {
    listen 443 ssl http2;
    server_name app.example.com app.example.net;
    ssl_certificate     /etc/letsencrypt/live/myapp/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/myapp/privkey.pem;
    # ... single proxy_pass block; Host $host passes the real hostname through
}
```

Mixing two different certs (a per-domain cert plus a shared wildcard) forces two
`server` blocks. Run `certbot certonly` first, then write the config that
references the new lineage — pointing `ssl_certificate` at a path that does not
exist yet makes `nginx -t` fail.

## Common failure modes

### `.next/standalone` missing

Cause:

- no `output: "standalone"` in Next config

Fix:

- add standalone output
- rebuild locally

### `server.js` missing on the server

Check:

- whether the SSR deploy path actually ran
- whether `path` points to the directory you inspected
- whether `rsync` uploaded to a different location

### Missing `public/`

`public/` is optional. The current deploy code skips it when the directory does not exist.

If assets are missing in production, check whether the app actually has a `public/` directory locally and whether the expected files were copied into the remote `public/` directory.

### `Cannot find module 'next'` or `502 Bad Gateway`

Cause:

- `.next/standalone/node_modules/*` contains `pnpm` symlinks
- rsync preserved the symlinks instead of copying their targets
- or the standalone output itself is incomplete and missing traced runtime files
- PM2 starts `server.js`, Node cannot resolve `next`, and nginx returns `502 Bad Gateway`

Fix:

- use the dedicated `nextjs-ssr` CLI path in current `smbcloud-cli`
- make sure the standalone upload uses `rsync --copy-links`
- verify `node .next/standalone/server.js` works locally before deploy
- if local standalone fails too, add `outputFileTracingIncludes` for the missing runtime files and rebuild
- then restart PM2 and recheck `pm2 logs <pm2_app>`

### `Cannot find module 'styled-jsx'`, `@swc/helpers`, or other pnpm peer deps

A common concrete failure looks like:

- `code: 'MODULE_NOT_FOUND'`
- `path: '/home/git/apps/web/<app>/node_modules/@swc/helpers'`

That usually means the standalone upload succeeded, but the top-level hoisted symlink for a scoped package was not recreated on the server.

Cause:

- pnpm stores peer dependencies inside `node_modules/.pnpm/<pkg>@<version>/node_modules/<pkg>` with internal symlinks that Node's standard `require.resolve()` cannot follow
- Next.js standalone traces the files into `.pnpm/` correctly, but does not create the flat `node_modules/<pkg>` entries needed for resolution
- this affects `styled-jsx`, `@swc/helpers`, `@next/env`, and other packages that `next` requires at startup

Fix:

- the CLI deploy script now auto-hoists in two passes:
  - first mirror pnpm's virtual directory at `node_modules/.pnpm/node_modules/*`
  - then fall back to scanning `node_modules/.pnpm/*/node_modules/*` for anything still missing
- this preserves pnpm's exact resolved package choice and fixes scoped packages like `@swc/helpers` as well as unscoped ones like `styled-jsx`
- no manual intervention needed for new deploys once the updated CLI is used
- to fix a server manually, prefer pnpm's virtual directory as the source of truth:
  - create `node_modules/@scope/` directories as needed
  - symlink `node_modules/<pkg>` or `node_modules/@scope/<pkg>` to the corresponding entry under `node_modules/.pnpm/node_modules/`
  - only if that directory is missing, scan individual `.pnpm/<store-entry>/node_modules/*` folders as a fallback

### Wrong port after deploy

Cause:

- `port` in `.smb/config.toml`, server ecosystem config (`.cjs` or `.js`), and nginx upstream disagree

Fix:

- standardize the same port in all three places
- if using ecosystem config, make sure PM2 is actually starting from that file

### PM2 "module is not defined" or "require is not defined"

Cause:

- the project's `package.json` has `"type": "module"`, which makes Node treat `.js` files as ESM
- `ecosystem.config.js` uses `module.exports` (CommonJS), which is invalid under ESM
- deploy shims that use `require()` will also fail under ESM

Fix:

- rename `ecosystem.config.js` to `ecosystem.config.cjs` on the server
- deploy shims must use `import()` not `require()`
- the CLI now handles both automatically: it checks for `.cjs` first and the server-side migration auto-renames `.js` to `.cjs` during deploy

### `rsync` of `.next/standalone/` fails with status 23 (dangling pnpm symlink)

A reproducible failure on pnpm monorepos. The deploy aborts at the standalone
upload with an empty stderr and:

```
✘ rsync of '.next/standalone/' failed (status 23):
```

Run the same rsync by hand and you see the real cause:

```
IO error encountered -- skipping file deletion
```

Cause:

- Next.js standalone tracing emits a virtual pnpm symlink such as
  `.next/standalone/node_modules/.pnpm/node_modules/semver -> ../semver@6.3.1/node_modules/semver`,
  but only the version it actually traced (e.g. `semver@7.7.3`) is copied into
  the bundle. The `@6.3.1` target is never written, so the symlink dangles.
- The CLI rsyncs standalone with `--copy-links`, which tries to follow every
  symlink. A dangling link is an IO error → rsync exits 23 and skips deletion.
- The package is not part of the traced runtime, so the link is safe to drop.

Find them:

```sh
find .next/standalone/ -type l ! -exec test -e {} \; -print
```

Fix:

- The CLI now handles this automatically. `process_deploy_nextjs_ssr` runs a
  step 3b (`prune_dangling_symlinks`) after the build and before the standalone
  rsync: it walks `.next/standalone/` and deletes any symlink whose target does
  not exist (real dirs are traversed; symlinked dirs are never followed). When
  it removes anything it prints `Pruned N dangling symlink(s) …`. No manual step
  is needed on current `smbcloud-cli`.
- Older `smb` (≤ 0.4.7) lacks the prune. On those, remove the link(s) before
  upload and re-run the deploy — but the rebuild regenerates them, so it must
  happen after `next build` and before rsync:
  `find .next/standalone/ -type l ! -exec test -e {} \; -delete`
  Driving the deploy by hand, delete then run the three rsyncs yourself. A
  repo-local equivalent is a `postbuild` script that runs the same `find …
  -delete`, since `smb` invokes `pnpm build` (which fires `postbuild`) in the
  right window.

### Misleading deploy output

The CLI may print generic deploy messages such as:

- `Deploying > Building the app`
- `App is running`

These messages do not mean git push happened. Confirm behavior from the code path, not the spinner text.

## Auth and production-only web issues

If the Next.js app uses the wasm auth client from `smbcloud-auth-sdk-wasm`, production failures that show:

- `Network error. Please check your internet connection and try again.`

are often not actual connectivity failures.

Check these first:

- Rails CORS allows the production origin
- `Authorization` response header is exposed over CORS
- backend auth status codes match the Rust parser logic

### Rails requirement

For browser clients using `POST /v1/client/users/sign_in`, if the token is returned in the `Authorization` response header, CORS must include:

- production origins such as `https://ondeinference.com`
- `expose: %w[... Authorization]`

Without that, browser fetch in wasm can surface as a fake network error.

### Rust requirement

The shared Rust networking layer must not collapse all non-200 auth responses into `NetworkError`.

If backend returns meaningful auth errors such as `401` invalid password, update the parser in `request_login` to preserve the real failure.

## Recommended validation

Use the smallest checks that prove the contract.

### Next.js app

- `pnpm build`
- verify standalone contains a runnable server entrypoint
- if `.next/standalone/server.js` exists, verify `node .next/standalone/server.js` starts locally
- if standalone preserves the source path, verify `node .next/standalone/<source>/server.js` starts locally

### smbcloud-cli

- `cargo check -p smbcloud-network`
- `cargo check -p cli` when deploy code changed

### Server

- `pm2 list`
- `pm2 logs <pm2_app>`
- `ls -la /home/git/apps/web/<app>`
- `ls -ld /home/git/apps/web/<app>/node_modules/next`
- `sudo nginx -t`

## Common mistakes

- assuming `deployment_method = 1` alone activates SSR deploy
- forgetting `kind = "nextjs-ssr"`
- keeping an old git `post-receive` hook and expecting it to manage SSR deploys
- `alias`-ing `/_next/static` for a nested (monorepo) standalone build — static
  is under `<app>/<source>/.next/static`, so the alias 404s; proxy everything instead
- assuming a dangling pnpm symlink in `.next/standalone/` still breaks the
  deploy — current `smbcloud-cli` prunes them in step 3b before the
  `rsync --copy-links` upload; only pre-prune manually on `smb` ≤ 0.4.7
- giving two hostnames one `server` block without a shared SAN cert
- using static-file Nginx config for an SSR app
- rsyncing `.next/standalone/` without `--copy-links` and shipping broken `pnpm` symlinks to the server
- editing the repo copy of `ecosystem.config.cjs` and expecting a normal CLI deploy to overwrite the server copy — it is preserved intentionally
- editing the server `ecosystem.config.cjs` but the deploy script falling back to inline `node server.js` because the file path check is wrong
- keeping an `ecosystem.config.js` when the project uses ESM (`"type": "module"` in `package.json`) — rename it to `.cjs` or let the CLI migration handle it
- leaving port values inconsistent across PM2, Nginx, and `.smb/config.toml`
- forgetting to set `frontend_app_id` on the deploy target once app-level deployment tracking is available
- treating browser CORS failures as real internet outages
- deploying without `frontend_app_id` in config — the API will try to infer the app, but explicit is better than implicit
