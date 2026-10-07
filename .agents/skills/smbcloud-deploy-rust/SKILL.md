---
name: smbcloud-deploy-rust
description: Use when deploying or debugging Rust services on smbCloud with `kind = "rust"`, which cross-compiles a Linux binary locally, uploads it with rsync, and restarts it over SSH (nohup by default, or a git-owned `systemctl --user` service when `process_manager = "systemd"`), with a health check and automatic rollback to the previous binary.
---

# smbCloud Deploy Rust

Use this skill when work touches `kind = "rust"` deploy targets.

## What `smb deploy` does

1. Resolves `process_manager` and `health_path` (a bad value fails before the build).
2. Builds `--release --bin <binary_name>` for `rust_target` (cargo, `cargo-zigbuild` or `cross`).
3. rsyncs the binary to `<path>/<binary>.new`.
4. Runs a remote script that copies the live binary to `<binary>.previous`, moves `.new` into place, restarts the service, and checks it.
5. If the check fails and `.previous` exists, restores it, restarts, prints the last 20 log lines, and exits non-zero. The deployment is marked `Failed`.

## Config

```toml
[project]
kind = "rust"
path = "apps/rest-api/my-service"   # remote app directory
binary_name = "my-service"          # optional; defaults to the Cargo package name
rust_target = "x86_64-unknown-linux-gnu"  # optional
process_manager = "systemd"         # optional; "nohup" (default) or "systemd"
port = 8080                         # optional; used by the health check
health_path = "/health"             # optional; must start with "/"
```

- `process_manager`: unset, `""` and `"nohup"` keep the kill-and-nohup start. `"systemd"` restarts `<binary_name>.service`. Any other value is an error, so a typo cannot start a second process next to a supervised one.
- `health_path`: with `port` set, the script polls `http://127.0.0.1:<port><health_path>` with `curl -fsS` once a second for up to 30 seconds. Without both, the check is `systemctl --user is-active` after a short sleep (systemd) or `pidof` (nohup). Only letters, digits and `/ - _ . ~ %` are accepted.

## Remote layout

```
<path>/<binary>            live binary
<path>/<binary>.new        upload staging, moved into place by the deploy
<path>/<binary>.previous   copy of the binary that was live before this deploy
<path>/<binary>.log        nohup mode only
```

## Process manager: systemd --user

The deploy SSHes as the app user and runs `systemctl --user restart <binary_name>.service` with `XDG_RUNTIME_DIR=/run/user/$(id -u)`. No sudo. The unit is created out of band; a missing unit fails the deploy before the binary is swapped.

The graceful stop time is the unit's `TimeoutStopSec`. Set it above the longest stream the service holds.

Prerequisites, once per app:

1. Enable linger for the app user so the unit runs without a login and starts at boot: `sudo loginctl enable-linger <user>`.
2. Create the env file and the unit as that user:

   ```sh
   export XDG_RUNTIME_DIR="/run/user/$(id -u)"
   APP=my-service                  # == binary_name == the unit name
   APP_PATH=~/apps/rest-api/my-service   # == `path` in .smb/config.toml
   mkdir -p ~/.config/systemd/user

   install -m 600 /dev/null ~/.config/systemd/user/$APP.env
   # add the service's environment variables, one KEY=value per line

   cat > ~/.config/systemd/user/$APP.service <<EOF2
   [Unit]
   Description=$APP
   After=network-online.target

   [Service]
   Type=simple
   WorkingDirectory=$APP_PATH
   EnvironmentFile=%h/.config/systemd/user/$APP.env
   ExecStart=$APP_PATH/$APP
   Restart=on-failure
   RestartSec=2
   TimeoutStopSec=60

   [Install]
   WantedBy=default.target
   EOF2

   systemctl --user daemon-reload
   ```

3. Set `process_manager = "systemd"` in `.smb/config.toml`.

## Cutting over from nohup or a system unit

From a nohup process: stop it (`kill $(pidof <binary>)`), then `systemctl --user enable --now <binary>.service` and confirm `systemctl --user is-active`. Never run both; they fight over the port.

From a system unit (`/etc/systemd/system`): this mode does not use sudo, so move the service to a user unit first: stop and disable the system unit, create the user unit as above, enable it, then switch `process_manager`. Until then, keep deploying that service manually.

## Troubleshooting

- `unit ... not found`: the user unit does not exist or `daemon-reload` was not run.
- `Failed to connect to bus`: linger is not enabled for the app user.
- Health check times out: check `journalctl --user -u <binary>.service -n 50`, and that `port` matches what the service binds.
- Rolled back: the previous binary is serving again; the failed build is gone from the server, fix and redeploy.
- No `.previous` on the first deploy, so a failed first deploy cannot roll back.
