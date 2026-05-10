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

- `Project` = umbrella workspace in smbCloud
- `DeployRepo` = repo or monorepo root inside that workspace
- Next.js deploy target in `.smb/config.toml` = the deployable app unit
- deployment record = one release of that app

For `nextjs-ssr`, smbCloud does not use git-push deploy.

The CLI route in `process_deploy.rs` short-circuits on:

- `kind = "nextjs-ssr"`

That path builds locally, then:

- rsyncs `.next/standalone/` into the remote app directory root
- rsyncs `.next/static/` into `remote/.next/static/`
- rsyncs `public/` into `remote/public/`
- runs `ssh` and restarts the app with PM2

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
id = 28                    # workspace id
frontend_app_id = "uuid"   # deployable app id (preferred when available)
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
- `id` remains the umbrella workspace id used for auth and API routing
- `frontend_app_id` should be present when the API exposes it so deployment tracking is app-specific
- `source` is the local Next.js app directory
- `path` is the remote destination directory on the server
- `pm2_app` is mandatory because the CLI restarts PM2 by name
- `port` should be set explicitly and must match nginx
- `deployment_method = 1` may still be present, but the `kind` routing is what matters most

## Expected remote layout

After a successful deploy, the remote directory should look like:

- `/home/git/apps/web/<app>/server.js`
- `/home/git/apps/web/<app>/.next/static/...`
- `/home/git/apps/web/<app>/public/...`

Note that `server.js` is expected at the remote root because the CLI uploads the contents of `.next/standalone/`, not the folder itself.

## PM2 behavior

The CLI's deploy path now deletes and starts fresh, preferring the server's
`ecosystem.config.js` when it exists.

Conceptually it behaves like:

```sh
cd "$APP_PATH"
mkdir -p logs

if pm2 describe "$PM2_APP" > /dev/null 2>&1; then
  pm2 delete "$PM2_APP"
fi

if [ -f ecosystem.config.js ]; then
  pm2 start ecosystem.config.js --only "$PM2_APP" --env production
else
  NODE_ENV=production PORT=<port> HOSTNAME=127.0.0.1 pm2 start node --name "$PM2_APP" -- server.js
fi

pm2 save
```

This has two consequences:

- old git-based `post-receive` hooks are irrelevant for the SSR path
- the configured `port` in `.smb/config.toml` is used for the fallback start path
- the live server `ecosystem.config.js` is the runtime source of truth when present

### Environment variables â ecosystem file (standard pattern)

The server runs in a **multi-tenant setup**: multiple Next.js apps share one server, each managed as a separate PM2 process. The standard way to manage per-app environment variables is an `ecosystem.config.js` file kept **on the server only** (never committed to the app repo).

The flow works like this:

1. On first setup, SSH in, create the ecosystem file, start with it, and save:

   ```sh
   ssh git@<server>
   cd /home/git/apps/web/<app>

   cat > ecosystem.config.js << 'EOF'
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

   pm2 start ecosystem.config.js --env production
   pm2 save
   ```

2. Every subsequent CLI deploy deletes the PM2 process and starts it fresh. If the server has `ecosystem.config.js`, the CLI starts PM2 from that file again. If not, it falls back to `node server.js` with inline `NODE_ENV`, `PORT`, and `HOSTNAME`.

3. To change or add env vars, SSH in, edit `ecosystem.config.js`, then:

   ```sh
   pm2 restart <pm2_app> --update-env
   pm2 save
   ```

The ecosystem file is the server-side source of truth for each app's runtime config. The CLI preserves the server copy during rsync and must not delete it.

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

If an older `ecosystem.config.js` exists and contains both:

- `args: "start -p 3026"`
- `env_production.PORT = 3025`

that is a broken configuration. Standardize on one port value.

Current rules:

- fallback fresh-start path uses `port` from `.smb/config.toml`
- ecosystem-managed apps should set the same port in `env_production.PORT`
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

The current deploy code tries to rsync `public/` unconditionally.

If the project has no `public/` directory, deploy may fail. Either:

- create an empty `public/`
- or patch the CLI to skip missing directories

### Wrong port after deploy

Cause:

- `port` in `.smb/config.toml`, server `ecosystem.config.js`, and nginx upstream disagree

Fix:

- standardize the same port in all three places
- if using ecosystem config, make sure PM2 is actually starting from that file

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
- verify `.next/standalone/server.js` exists

### smbcloud-cli

- `cargo check -p smbcloud-network`
- `cargo check -p cli` when deploy code changed

### Server

- `pm2 list`
- `pm2 logs <pm2_app>`
- `ls -la /home/git/apps/web/<app>`
- `sudo nginx -t`

## Common mistakes

- assuming `deployment_method = 1` alone activates SSR deploy
- forgetting `kind = "nextjs-ssr"`
- keeping an old git `post-receive` hook and expecting it to manage SSR deploys
- using static-file Nginx config for an SSR app
- editing the repo copy of `ecosystem.config.js` and expecting a normal CLI deploy to overwrite the server copy — it is preserved intentionally
- editing the server `ecosystem.config.js` but the deploy script falling back to inline `node server.js` because the file path check is wrong
- leaving port values inconsistent across PM2, Nginx, and `.smb/config.toml`
- forgetting to set `frontend_app_id` on the deploy target once app-level deployment tracking is available
- treating browser CORS failures as real internet outages
