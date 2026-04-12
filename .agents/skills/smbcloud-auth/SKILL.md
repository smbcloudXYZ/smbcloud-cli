---
name: smbcloud-auth
description: Use when building, debugging, or extending smbCloud authentication across the Rails auth service, the web console, the Rust or WASM auth SDKs, and Tauri or web client apps. Covers platform clients vs tenant auth apps, AuthUser signup/sign-in/confirmation, OAuth providers for AuthUser, shared auth apps across projects, browser SDK integration, and development vs production app credentials.
---

# smbCloud Auth

Use this skill when work touches any part of the smbCloud auth stack:

- `smbcloud-api` Rails auth service
- `smbcloud-web-console` Next.js admin UI
- `smbcloud-cli/crates/smbcloud-auth` Rust SDK
- `smbcloud-cli/crates/smbcloud-auth-sdk-wasm` browser SDK
- Tauri apps such as Rumi Learn Persian and PBJ Komplit
- web apps that use `@smbcloud/sdk-auth`

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

### Provider auth boundary

OAuth providers such as Apple or Google for customer apps belong to the tenant auth plane:

- provider sign-in for end users must resolve to `AuthUser`
- provider linkage must be scoped to `AuthApp`
- the result must be the same tenant auth session shape used by email/password

Do not route tenant provider auth through:

- platform `User`
- `Authorization` records for platform users
- Doorkeeper/OIDC smbCloud internal users

If provider support is added for tenant apps, the backend contract should end in an `AuthUser` session or JWT, not a provider token as the primary app session.

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

### Tenant provider auth modeling

If tenant users need Apple, Google, or other OAuth providers:

- add provider linkage for `AuthUser`, not platform `User`
- model by stable provider subject, not by email
- keep provider records scoped to the tenant auth app

Preferred shape:

- `AuthUserAuthorization`
  - `auth_user_id`
  - `provider`
  - `uid`
  - optional `email`
  - optional `raw_profile`

For Apple specifically:

- use Apple `sub` as the stable provider identity
- do not treat email as the primary key
- do not assume name is available after first sign-in

The Rails backend should verify provider assertions and then mint the normal tenant auth session for the resolved `AuthUser`.

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
- `smbcloud-cli/crates/smbcloud-auth-sdk-wasm`

When changing tenant app auth:

- keep tenant auth on `/v1/client/*`
- preserve `AccountStatus` mapping:
  - `NotFound`
  - `Ready { access_token }`
  - `Incomplete { status }`
- `request_login` depends on `422` + `error_code` for incomplete accounts

If the backend changes error shape or status codes, update the Rust parser accordingly.

### Browser/WASM SDK conventions

The browser package is consumed as `@smbcloud/sdk-auth`.

Current exported surface includes:

- `signup_with_client`
- `login_with_client`
- `me_with_client`
- `logout_with_client`
- `remove_with_client`
- `Environment.Dev`
- `Environment.Production`

Recommended browser integration pattern:

- lazy-load the WASM runtime with `await import("@smbcloud/sdk-auth")`
- call `await runtime.default()` before using the exported functions
- store only the access token in browser storage
- restore the session with `me_with_client`
- gate environment switching behind explicit debug controls

Do not build browser flows around NextAuth sessions if the product session is actually an smbCloud `AuthUser` token. The browser app should either:

- use the browser SDK directly for email/password flows, or
- complete provider auth through Rails and then store the resulting tenant auth token

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

### Public-client security rule

Tauri, mobile, and browser apps are public clients. Treat `app_id` as public and `app_secret` as non-confidential if it is shipped in a client binary or browser bundle.

Implications:

- do not claim the client app secret is secure once distributed
- do not rely on embedded `app_secret` as the main client authentication boundary
- prefer backend-mediated flows, PKCE, or other public-client-safe patterns for stronger security

If the current SDK or API still requires `app_secret` from public clients, call out that limitation explicitly in code review or implementation notes.

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
3. If OAuth providers are involved, confirm whether the provider is for `AuthUser` or internal `User`. Default tenant/customer providers to `AuthUser`.
4. Update Rust or WASM SDK endpoints/parsing if the API contract changed.
5. Update Tauri or browser clients to match.
6. Update the web console if auth app management changed.
7. Validate end-to-end.

## Validation

Use the smallest relevant checks.

### Rails

- `bundle exec rails routes | grep 'v1/client\|auth_apps'`
- `ruby -c` on touched controller/model/mailer files
- `bundle exec rails db:migrate` if schema changed

### Rust SDK

- `cargo check -p smbcloud-auth`
- `cargo check -p smbcloud-auth-sdk-wasm`

### Tauri apps

- `cargo check` in `src-tauri`
- `pnpm build` in the app root when frontend changed

### Web console

- `pnpm build`

### Browser SDK apps

- `pnpm build`
- verify login, logout, and session restore in the browser
- if environment override exists, verify both development and production modes

## Common mistakes

- Sending tenant app credentials to `/v1/users`
- Using `OauthApplication` instead of `AuthApp` for tenant login
- Replacing platform client validation with tenant auth app validation
- Implementing Apple or Google for tenant users on platform `User` instead of `AuthUser`
- Returning raw provider tokens as the primary tenant app session
- Treating a browser or native app `app_secret` as confidential
- Auto-confirming `AuthUser` while telling the client to check email
- Building smbCloud `AuthUser` web auth on top of unrelated NextAuth session state
- Forgetting that `tauri dev` ignores `tauri.conf.json` macOS minimum version
