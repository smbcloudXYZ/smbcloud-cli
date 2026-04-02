---
name: smbcloud-deploy-rsync
description: Use when implementing or debugging smbCloud's generic rsync-based deployment path in `smbcloud-cli`, especially `rsync_deploy.rs`, workspace dependency wiring, SSH identity handling, remote path normalization, and parity with shell rsync commands like `rsync -a source/ host:path/`.
---

# smbCloud Deploy Rsync

Use this skill when work is about smbCloud's generic rsync deploy path in the Rust CLI, not the specialized Next.js SSR flow.

Applies to:

- `crates/cli/src/deploy/rsync_deploy.rs`
- workspace dependency wiring in `Cargo.toml`
- embedding the Rust rsync crate instead of shelling out to system `rsync`
- reproducing command semantics like `rsync -a source/ host:path/`
- SSH key and remote destination handling for smbCloud runners

## Scope boundary

This skill is for the generic rsync deploy implementation in `smbcloud-cli`.

Do not use it for:

- `kind = "nextjs-ssr"` local-build deploys
- PM2 restart logic for standalone Next.js bundles
- Nginx config for SSR apps

Those belong to the Next.js deploy skill.

## Target behavior

The baseline command to replicate is:

```sh
rsync -a 5mb-dot-app/ rsync-deploy-id-11:apps/web/5mb.app/
```

Important semantics:

- trailing slash on the source means copy the contents of the directory
- trailing slash on the destination means sync into that directory
- archive mode preserves the usual rsync metadata behavior
- the remote target is `host:path`, not a local filesystem path

When implementing in Rust, preserve these semantics explicitly. Do not assume path handling will match shell rsync unless you normalize it yourself.

## Current smbCloud conventions

Generic rsync deploy uses smbCloud config and runner metadata.

Expect these responsibilities:

- get the logged-in smbCloud user
- resolve the SSH identity path from `Config::ssh_key_path(user.id)`
- determine the runner host, usually through `runner.rsync_host()`
- build the remote destination from configured project `path`
- deploy the current source tree or configured source directory

The user-visible output may still mention the SSH key path. Preserve useful diagnostics, but avoid misleading claims about git deploy.

## Dependency wiring

When the task is to replace shell `rsync` with the Rust embedding crate:

1. add the upstream rsync embedding crate at the workspace root `Cargo.toml`
2. add it as a workspace dependency in the consuming crate
3. wire only the minimal surface needed by `rsync_deploy.rs`

Prefer the embedding crate over copying CLI subprocess behavior when the goal is portability or tighter control.

Before coding, inspect:

- the embedding crate API surface
- whether it supports client mode directly
- how stdout/stderr and exit status are reported
- how remote-shell transport is configured, if at all

If the crate cannot actually express `host:path` SSH transport parity, say so explicitly instead of forcing a partial migration.

## Implementation guidance

### Path normalization

Be precise with trailing slashes.

Preferred rule:

- normalize local source to end with `/`
- normalize remote destination to end with `/`

That matches the intended `rsync -a source/ host:path/` behavior.

### SSH identity

The CLI currently resolves the private key path from the smbCloud user id.

If the embedded rsync client still needs an ssh transport command, keep the identity handling equivalent to the current shell behavior.

Typical concerns:

- identity file path
- strict host key checking
- known hosts file or pinned host key strategy
- batch mode / no password prompts

### Failure reporting

Preserve actionable errors.

Good failures include:

- local source path missing
- remote project path missing in config
- SSH key missing
- embedded client non-zero exit status
- stderr emitted by the transfer implementation

Do not flatten all deploy failures into a generic message.

### Behavior parity

If replacing subprocess `rsync`, confirm parity for:

- source contents vs source directory copying
- remote path handling
- recursive transfer
- deletion behavior, if smbCloud depends on it
- stdout/stderr capture
- non-zero status mapping

## Validation

Use small checks first.

### Rust

- `cargo check -p smbcloud-cli`
- targeted tests if added around path normalization or command construction

### Behavioral

Verify the resulting implementation matches the shell command shape conceptually:

- source `dir/`
- destination `host:path/`
- ssh identity derived from smbCloud config

If the code still shells out to `rsync`, validate the exact arguments.
If it migrates to the embedding crate, validate equivalent output and error handling.

## Common mistakes

- mixing this generic rsync deploy path with the specialized `nextjs-ssr` flow
- losing trailing slash semantics and copying the wrong directory level
- adding the dependency in one `Cargo.toml` but not wiring the workspace dependency correctly
- assuming the embedding crate automatically supports ssh remote syntax without checking
- hiding transport stderr that the user needs to debug deploy failures
- hardcoding local paths instead of using smbCloud config and user-derived SSH identity
