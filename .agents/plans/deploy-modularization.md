# Plan: decouple build/deploy from the CLI into `smbcloud-deploy`

Status: in progress. Stages 0 to 3 (vite_spa slice) shipped. Remaining work below
is the **next milestone**.

## Goal

Move the deploy logic out of `crates/cli` into a reusable `crates/smbcloud-deploy`
engine so the CLI, the CI action, and (later) the server-side git receiver all
drive the same code. They differ only in how they report progress and how they
authenticate.

Two inversions make it reusable:
- `Reporter` replaces direct `spinners` / `dialoguer` / `println!`, so the engine
  never owns the terminal. The CLI supplies `SpinnerReporter`.
- Auth is passed in (a token / credentials); the engine never reads `~/.smb` or
  prompts for login.

Decisions already made:
- Networking crates (`smbcloud-networking*`) stay direct deps of the engine.
  Only reporting and auth are inverted.
- `git.rs` (SSH-git remote setup) stays in the CLI for now. It is the SSH-git
  path slated for replacement by git-smart-HTTP, so decoupling it is wasted work.
- Interactive setup (`setup*.rs`, `process_migrate.rs`) stays in the CLI. It is
  UX, not engine.

## Done

- **Stage 0** engine crate: `error::DeployError`, `report::{Reporter, NoopReporter}`,
  `runner::detect_runner`.
- **Stage 1** CLI depends on the engine; `ui/reporter.rs::SpinnerReporter`;
  `detect_runner` rewired; old `deploy/detect_runner.rs` removed.
- **Stage 2** transport: `transport::{Transport, RsyncTransport}` + moved
  `known_hosts`. CLI helper `deploy::rsync_transport(config, runner, user_id)`
  resolves the local `~/.ssh` identity and remote path.
- **Stage 3 (vite_spa slice)** build split: `build::{BuildStrategy, BuildArtifact,
  ViteSpaBuild}`. `process_deploy_vite_spa.rs` now calls the engine for the build
  and reuses one `SpinnerReporter` for build + transport.

## Next milestone

### 1. Shared remote-command step

Several strategies do the same "SSH in and restart the service" step, inlined
per file. Extract it into the engine first (e.g. `transport::RemoteCommand` or a
`remote` module) so the strategies can share it. Model it behind the same pinned
host-key SSH used by `RsyncTransport`.

### 2. Remaining `BuildStrategy` extractions (one per pass, compile-green each)

Do smallest first. Each: lift build mechanics into a `BuildStrategy`, swap
`spinners` + `stop_and_persist` for `Reporter`, take auth as a param, route the
remote-restart step through step 1.

| File | Lines | Shape |
| --- | --- | --- |
| `process_deploy_rails.rs` | ~395 | rsync shared lib, SSH compile native gem, git force-push |
| `process_deploy_rust.rs` | ~641 | cross-build Linux binary, rsync, SSH restart |
| `process_deploy_nextjs_ssr.rs` | ~700 | install, build, upload `.next/standalone`, SSH restart |
| `process_deploy_swift.rs` | ~832 | Docker Linux build, rsync binary + Resources, SSH restart |

Follow `ViteSpaBuild` as the template. Keep the deployment-record API calls
(`create_deployment` / `update`) in the CLI router, not the engine.

### 3. Stage 4: router + retire the CommandResult wart

- Flip `process_deploy.rs`'s router to call an `engine::deploy(...)` orchestrator.
- Retire `CommandResult`'s `spinner: Spinner` field (the "return a live spinner,
  caller stops it" pattern) now that the engine reports the whole flow through
  `Reporter`. `CommandResult` has ~50 construction sites, so this is its own pass:
  either make `spinner` optional or drop it and have commands return plain data.

## Reference

- Spec: `smbcloud` repo `.agents/specs/smbcloud-deploy.md` (product direction),
  `smbcloud-actions.md` (CI wrapper). Direction is git-smart-HTTP, local-first
  build, drop SSH transport later.
- Engine crate: `crates/smbcloud-deploy/`.
