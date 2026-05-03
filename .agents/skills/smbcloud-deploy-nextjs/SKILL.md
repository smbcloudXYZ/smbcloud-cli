---
name: smbcloud-deploy-nextjs
description: Use when deploying or debugging Next.js apps on smbCloud, especially the dedicated `nextjs-ssr` flow that builds locally, uploads `.next/standalone` via rsync, restarts PM2 over SSH, and serves traffic behind Nginx.
---

# smbCloud Deploy Next.js

Use this skill when work touches Next.js deployment on smbCloud.

Applies to:

- `kind = "nextjs-ssr"` projects
- `.smb/config.toml` deploy configuration
- `process_deploy_nextjs_ssr.rs`
- PM2 process management for standalone builds
- Nginx proxying for SSR apps
- production-only issues caused by CORS, ports, or runtime mismatch

## Deployment model

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
deployment_method = 1
kind = "nextjs-ssr"
source = "."
package_manager = "pnpm"
pm2_app = "my-app"
path = "apps/web/my-app"
```

Important rules:

- `kind = "nextjs-ssr"` is what activates the SSR deploy path
- `source` is the local Next.js app directory
- `path` is the remote destination directory on the server
- `pm2_app` is mandatory because the CLI restarts PM2 by name
- `deployment_method = 1` may still be present, but the `kind` routing is what matters most

## Expected remote layout

After a successful deploy, the remote directory should look like:

- `/home/git/apps/web/<app>/server.js`
- `/home/git/apps/web/<app>/.next/static/...`
- `/home/git/apps/web/<app>/public/...`

Note that `server.js` is expected at the remote root because the CLI uploads the contents of `.next/standalone/`, not the folder itself.

## PM2 behavior

The CLI's deploy path runs logic equivalent to:

```sh
cd "$APP_PATH"

if pm2 describe "$PM2_APP" > /dev/null 2>&1; then
  pm2 restart "$PM2_APP"
else
  PORT=3010 HOSTNAME=127.0.0.1 pm2 start node --name "$PM2_APP" -- server.js
fi

pm2 save
```

This has two consequences:

- old git-based `post-receive` hooks are irrelevant for the SSR path
- the port is currently hardcoded to `3010` in the CLI for fresh starts unless patched

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

2. Every subsequent CLI deploy runs `pm2 restart <pm2_app>`. Because `pm2 save` already persisted the env, PM2 restarts with the same environment â **the CLI does not need to know about the ecosystem file at all**.

3. To change or add env vars, SSH in, edit `ecosystem.config.js`, then:

   ```sh
   pm2 restart <pm2_app> --update-env
   pm2 save
   ```

The ecosystem file is the server-side source of truth for each appâs runtime config. Future versions of the smbCloud web console will provide a UI to manage these vars per app, which will write to this file and trigger the restart remotely.

### Manual PM2 operations (without ecosystem file)

If the app is already deployed and `server.js` exists:

```sh
cd /home/git/apps/web/<app>
pm2 restart <pm2_app> --update-env
pm2 save
```

If starting from scratch without an ecosystem file:

```sh
cd /home/git/apps/web/<app>
PORT=<port> HOSTNAME=127.0.0.1 pm2 start node --name <pm2_app> -- server.js
pm2 save
```

## Port management

Use one source of truth for the runtime port.

If an older `ecosystem.config.js` exists and contains both:

- `args: "start -p 3026"`
- `env_production.PORT = 3025`

that is a broken configuration. Standardize on one port value.

For the current SSR deploy code, the real runtime is controlled by the environment passed to `node server.js`, not by `next start` or an ecosystem file.

If the app must run on a port other than `3010`, patch the CLI or restart PM2 manually with the correct port.

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

- CLI hardcodes `PORT=3010` for fresh starts

Fix:

- manually restart with the correct port
- or patch the CLI to read a configurable port from `.smb/config.toml`

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
- editing `ecosystem.config.js` but forgetting `pm2 restart <app> --update-env && pm2 save` â PM2 keeps the old env until explicitly refreshed
- leaving port values inconsistent across PM2, Nginx, and CLI
- treating browser CORS failures as real internet outages
