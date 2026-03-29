---
name: smbcloud-auth
description: Use when building, debugging, or extending smbCloud authentication across the Rails auth service, the web console, the Rust networking SDK, and Tauri client apps. Covers platform clients vs tenant auth apps, AuthUser signup/sign-in/confirmation, shared auth apps across projects, and development vs production app credentials.
---

# smbCloud Auth

Use this skill when work touches any part of the smbCloud auth stack:

- `smbcloud-api` Rails auth service
- `smbcloud-web-console` Next.js admin UI
- `smbcloud-cli/crates/smbcloud-auth` Rust SDK
- Tauri apps such as Rumi Learn Persian and PBJ Komplit

## Core model

There are two auth planes. Do not mix them.

### 1. Platform clients

These are smbCloud's own apps:

- `web`
- `cli`
- `sigit`
- `moovibe`
- `webconsole`

They authenticate smbCloud operator users through platform endpoints such as:

- `POST /v1/users/sign_in`
- `POST /v1/users`

They must use the platform client validator, not tenant app validation.

### 2. Tenant auth apps

These are customer-facing apps such as Rumi or PBJ Komplit.

They authenticate end users through:

- `POST /v1/client/users`
- `POST /v1/client/users/sign_in`
- `GET /v1/client/me`
- `DELETE /v1/client/me`
- `DELETE /v1/client/users/sign_out`
- `GET /v1/client/users/confirmation`

The tenant auth domain is:

- `AuthApp`
- `AuthAppSecret`
- `AuthUser`

Tenant apps must use `AuthAppClientAuthenticatable` and must not go through platform `/v1/users`.

## Multi-project auth apps

Current rule:

- one `AuthApp` has an owner project via `auth_apps.project_id`
- one `AuthApp` can be shared with other projects through `auth_app_projects`
- one project can point to only one auth app

If a workspace/project should reuse an existing auth app, attach it instead of creating a duplicate.

## Rails conventions

When editing `smbcloud-api`:

- Platform operator auth stays under `User`
- Tenant app auth stays under `AuthUser`
- Do not reuse `OauthApplication` for tenant end-user login
- Do not hardcode tenant app validation in platform controllers

### AuthUser signup/sign-in rules

- Signup creates an `AuthUser` scoped to `current_auth_app`
- Signup sends confirmation email
- Do not auto-set `confirmed_at`
- Sign-in returns `EmailNotVerified` until email is confirmed
- Deletion removes the currently authenticated `AuthUser`

### Mail flow

For tenant users, use the explicit confirmation flow already established:

- custom mailer for `AuthUser`
- confirmation link to `/v1/client/users/confirmation?confirmation_token=...`

Do not claim email confirmation exists unless the mailer and route are actually wired.

## Rust SDK conventions

The Rust client surface lives in:

- `smbcloud-cli/crates/smbcloud-auth`

When changing tenant app auth:

- keep tenant auth on `/v1/client/*`
- preserve `AccountStatus` mapping:
  - `NotFound`
  - `Ready { access_token }`
  - `Incomplete { status }`
- `request_login` depends on `422` + `error_code` for incomplete accounts

If the backend changes error shape or status codes, update the Rust parser accordingly.

## Tauri client conventions

For Rumi/PBJ-style apps:

- use a dedicated `src-tauri/src/smbcloud/mod.rs`
- expose commands for:
  - `signup_user`
  - `sign_in_user`
  - `current_user`
  - `sign_out_user`
  - `delete_current_user`
  - `is_debug_build`
- store auth state in plugin-store:
  - access token
  - provider
  - auth environment
  - language

### Environment handling

Keep development and production app credentials separate.

Preferred pattern:

- development credentials for local auth service
- production credentials for hosted auth service
- switch via a Rust `AuthEnvironment` enum
- expose the switch in debug builds only

Recommended env variable names:

- `SMBCLOUD_APP_ID_DEV`
- `SMBCLOUD_APP_SECRET_DEV`
- `SMBCLOUD_APP_ID_PRODUCTION`
- `SMBCLOUD_APP_SECRET_PRODUCTION`

### macOS Tauri dev builds

`tauri.conf.json` `minimumSystemVersion` is not enough for `tauri dev`.

For apps that link Swift-based plugins such as `tauri-plugin-iap`, add `src-tauri/.cargo/config.toml` with:

- `MACOSX_DEPLOYMENT_TARGET=14.0`
- `-mmacosx-version-min=14.0` for both Apple targets
- host `CC` and `CXX` overrides to `/usr/bin/clang` and `/usr/bin/clang++`

If you see `libswift_Concurrency.dylib` at runtime, fix Cargo/macOS config first.

## Web console conventions

The admin UI lives in `smbcloud-web-console`.

Use the `Auth` area, not generic settings pages, for tenant auth app management.

Expected behavior:

- scope auth view to selected workspace/project
- if workspace has an auth app, show the app and its users
- if not, allow:
  - creating a new auth app
  - attaching an existing tenant auth app
- show app id in list views
- hide app secret by default on detail views
- show linked/shared workspaces on detail pages

## Typical change flow

1. Decide whether the change is platform auth or tenant auth.
2. Update Rails first.
3. Update Rust SDK endpoints/parsing if the API contract changed.
4. Update Tauri clients to match.
5. Update the web console if auth app management changed.
6. Validate end-to-end.

## Validation

Use the smallest relevant checks.

### Rails

- `bundle exec rails routes | grep 'v1/client\|auth_apps'`
- `ruby -c` on touched controller/model/mailer files
- `bundle exec rails db:migrate` if schema changed

### Rust SDK

- `cargo check -p smbcloud-auth`

### Tauri apps

- `cargo check` in `src-tauri`
- `pnpm build` in the app root when frontend changed

### Web console

- `pnpm build`

## Common mistakes

- Sending tenant app credentials to `/v1/users`
- Using `OauthApplication` instead of `AuthApp` for tenant login
- Replacing platform client validation with tenant auth app validation
- Auto-confirming `AuthUser` while telling the client to check email
- Forgetting that `tauri dev` ignores `tauri.conf.json` macOS minimum version
