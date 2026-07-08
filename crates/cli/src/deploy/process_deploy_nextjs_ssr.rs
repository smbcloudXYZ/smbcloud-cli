use {
    crate::{
        cli::CommandResult,
        client,
        deploy::known_hosts,
        ui::{fail_message, fail_symbol, succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Result},
    chrono::Utc,
    smbcloud_auth::me::me,
    smbcloud_model::project::{DeploymentPayload, DeploymentStatus},
    smbcloud_network::environment::Environment,
    smbcloud_networking_project::{
        crud_project_deployment_create::create_deployment, crud_project_deployment_update::update,
    },
    smbcloud_utils::config::Config,
    spinners::{Spinner, Spinners},
    std::{
        io::Write,
        process::{Command, Stdio},
    },
    tempfile::NamedTempFile,
};

/// Deploys a Next.js SSR app using standalone output mode.
///
/// Requires `output: 'standalone'` in next.config.js. This produces a
/// self-contained `.next/standalone/` directory that includes only the
/// production Node.js dependencies needed to run the server — no
/// project-level `node_modules` upload required.
///
/// Steps:
///   1. `pnpm install --ignore-scripts`
///   2. `pnpm build`
///   3. POST deployment record as Started
///   4. rsync .next/standalone/  → server:path/          (server + bundled deps)
///   5. rsync .next/static/      → server:path/.next/static/  (static chunks)
///   6. rsync public/            → server:path/public/   (public assets)
///   7. SSH: pm2 delete + start fresh (prefer server `ecosystem.config.cjs` or `.js` if present)
///   8. PATCH deployment record as Done
pub async fn process_deploy_nextjs_ssr(env: Environment, config: Config) -> Result<CommandResult> {
    let source = config.project.source.as_deref().unwrap_or(".");
    let package_manager = config.project.package_manager.as_deref().unwrap_or("pnpm");

    let remote_path = config.project.path.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "path not set in .smb/config.toml (e.g. path = \"apps/web/myapp\")"
        ))
    })?;

    let pm2_app = config.project.pm2_app.as_deref().ok_or_else(|| {
        anyhow!(fail_message(
            "pm2_app not set in .smb/config.toml (e.g. pm2_app = \"my-app\")"
        ))
    })?;

    let access_token = crate::token::get_smb_token::get_smb_token(env)?;
    let user = me(env, client(), &access_token).await?;

    // ── Step 1: pnpm install --ignore-scripts ────────────────────────────────

    let source_dir = std::path::Path::new(source);
    if !source_dir.exists() {
        return Err(anyhow!(fail_message(&format!(
            "Source path '{}' does not exist. Check the 'source' field in .smb/config.toml.",
            source
        ))));
    }

    let mut install_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        succeed_message(&format!("Installing dependencies in {}…", source)),
    );

    // Capture stdout/stderr so pnpm's output does not interleave with the
    // spinner animation. On failure the captured output is printed for the user.
    let install_output = Command::new(package_manager)
        .args(["install", "--ignore-scripts"])
        .current_dir(source)
        .output()
        .map_err(|e| {
            anyhow!(fail_message(&format!(
                "Failed to spawn '{} install': {}",
                package_manager, e
            )))
        })?;

    if !install_output.status.success() {
        install_spinner.stop_and_persist(&fail_symbol(), fail_message("Install failed."));
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        let details = if !stderr.trim().is_empty() {
            stderr
        } else {
            stdout
        };
        if !details.trim().is_empty() {
            eprintln!("{}", details.trim());
        }
        return Err(anyhow!(fail_message(&format!(
            "'{} install --ignore-scripts' exited with status {}",
            package_manager, install_output.status
        ))));
    }

    install_spinner.stop_and_persist(
        &succeed_symbol(),
        succeed_message("Dependencies installed."),
    );

    // ── Step 2: pnpm build ───────────────────────────────────────────────────
    // No spinner — Next.js writes its own rich progress output to the terminal.
    // Blank lines before and after keep it visually separated from our log lines.

    println!();

    let build_status = Command::new(package_manager)
        .arg("build")
        .current_dir(source)
        .status()
        .map_err(|e| {
            anyhow!(fail_message(&format!(
                "Failed to spawn '{} build': {}",
                package_manager, e
            )))
        })?;

    println!();

    if !build_status.success() {
        return Err(anyhow!(fail_message(&format!(
            "'{} build' exited with status {}",
            package_manager, build_status
        ))));
    }

    println!(
        "{} {}",
        succeed_symbol(),
        succeed_message("Build complete.")
    );

    // ── Step 3: verify standalone output exists ──────────────────────────────
    //
    // If output: 'standalone' is missing from next.config.js the build
    // succeeds but .next/standalone/ is never created.

    let standalone_dir = format!("{}/.next/standalone", source);
    let standalone_path = std::path::Path::new(&standalone_dir);
    if !standalone_path.exists() {
        return Err(anyhow!(fail_message(
            ".next/standalone not found. Add `output: 'standalone'` to next.config.js and rebuild."
        )));
    }

    let runtime_subdir = if source != "." && standalone_path.join(source).join("server.js").exists()
    {
        Some(source.trim_end_matches('/').to_owned())
    } else {
        None
    };

    // ── Step 4: record deployment as Started ─────────────────────────────────

    let deploy_ref = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    let created_deployment = create_deployment(
        env,
        client(),
        &access_token,
        config.project.id,
        DeploymentPayload {
            commit_hash: deploy_ref.clone(),
            status: DeploymentStatus::Started,
            frontend_app_id: config.project.frontend_app_id.clone(),
        },
    )
    .await
    .ok();

    // ── Steps 5–7: rsync three items to the server ───────────────────────────
    //
    // Standalone mode produces everything needed to run the server:
    //
    //   .next/standalone/  — server.js + bundled production deps (no project node_modules upload)
    //   .next/static/      — client-side chunks (must be copied into standalone manually)
    //   public/            — static assets served directly by Next.js
    //
    // The static and public dirs must sit inside the standalone tree so
    // `node server.js` can find them at runtime.

    let runner = config.project.runner;
    let rsync_host = runner.rsync_host();

    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let identity_file = home.join(".ssh").join(format!("id_{}@smbcloud", user.id));
    let identity_file_str = identity_file.to_string_lossy().into_owned();

    // Write pinned known_hosts once for all rsync calls.
    let mut known_hosts_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(known_hosts_file, "{}", known_hosts::for_host(&rsync_host))
        .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    let ssh_command = format!(
        "ssh -i {identity} \
         -o StrictHostKeyChecking=yes \
         -o UserKnownHostsFile={known_hosts} \
         -o IdentitiesOnly=yes \
         -o PasswordAuthentication=no \
         -o BatchMode=yes",
        identity = identity_file_str,
        known_hosts = known_hosts_file.path().display(),
    );

    let remote_base = format!(
        "git@{}:{}",
        rsync_host,
        if remote_path.ends_with('/') {
            remote_path.to_owned()
        } else {
            format!("{}/", remote_path)
        }
    );

    struct Transfer {
        local_rel: &'static str,
        remote_rel: String,
        protect_runtime_files: bool,
        copy_links: bool,
    }

    let runtime_prefix = runtime_subdir
        .as_ref()
        .map(|path| format!("{}/", path))
        .unwrap_or_default();

    // Next.js copies the project's .env* files into .next/standalone/, so an
    // unfiltered upload would ship the developer's local (often development)
    // env straight into the production runtime directory — and `--delete`
    // would remove any operator-managed .env on the server. Env files are
    // excluded on both sides: local ones never upload, server ones survive
    // (rsync does not delete excluded destination files without
    // --delete-excluded). Runtime env is server-managed — ecosystem config or
    // a server-side .env. Anchored to the app roots so bundled node_modules
    // content is not affected.
    let env_file_excludes: Vec<String> = {
        let mut patterns = vec!["/.env*".to_string()];
        if !runtime_prefix.is_empty() {
            patterns.push(format!("/{}.env*", runtime_prefix));
        }
        patterns
    };

    let local_env_file = [
        format!("{}/.env", standalone_dir),
        format!("{}/{}.env", standalone_dir, runtime_prefix),
    ]
    .iter()
    .any(|path| std::path::Path::new(path).exists());
    if local_env_file {
        println!(
            "{} {}",
            succeed_symbol(),
            succeed_message(
                "Local .env* files found in the standalone build — not uploaded. \
                 Runtime env comes from the server (ecosystem config or server-side .env).",
            )
        );
    }

    // (local_source, remote_destination)
    // .next/standalone contents go to the root of remote_path.
    // .next/static and public go into the runtime directory that contains
    // server.js. When outputFileTracingRoot points above the app directory,
    // Next preserves the source path inside `.next/standalone/`.
    let transfers = vec![
        // standalone contents → remote root
        Transfer {
            local_rel: ".next/standalone/",
            remote_rel: String::new(),
            // pnpm leaves symlinked package entries in standalone output. The
            // server only receives this tree, so those symlinks must be
            // dereferenced during upload.
            copy_links: true,
            // The server copy of ecosystem.config.cjs (or .js) is operator-managed
            // runtime config and must survive deploys. Without this protection,
            // rsync `--delete` removes it because it does not exist in
            // .next/standalone/.
            protect_runtime_files: true,
        },
        // static chunks → runtime/.next/static/
        Transfer {
            local_rel: ".next/static/",
            remote_rel: format!("{}.next/static/", runtime_prefix),
            protect_runtime_files: false,
            copy_links: false,
        },
        // public assets → runtime/public/
        Transfer {
            local_rel: "public/",
            remote_rel: format!("{}public/", runtime_prefix),
            protect_runtime_files: false,
            copy_links: false,
        },
    ];

    let mut upload_spinner = Spinner::new(
        Spinners::Hamburger,
        succeed_message(&format!("Uploading to {}…", remote_path)),
    );

    for transfer in transfers {
        let local_path = format!("{}/{}", source, transfer.local_rel);
        let destination = format!("{}{}", remote_base, transfer.remote_rel);

        if !std::path::Path::new(&local_path).exists() {
            continue;
        }

        let mut rsync_args = vec!["-az".to_string(), "--delete".to_string()];

        if transfer.copy_links {
            rsync_args.push("--copy-links".to_string());
        }

        if transfer.protect_runtime_files {
            rsync_args.extend([
                "--exclude".to_string(),
                "ecosystem.config.js".to_string(),
                "--exclude".to_string(),
                "ecosystem.config.cjs".to_string(),
                "--exclude".to_string(),
                "logs/".to_string(),
            ]);
            for pattern in &env_file_excludes {
                rsync_args.extend(["--exclude".to_string(), pattern.clone()]);
            }
        }

        rsync_args.extend([
            "-e".to_string(),
            ssh_command.clone(),
            local_path.clone(),
            destination,
        ]);

        let output = Command::new("rsync")
            .args(&rsync_args)
            .output()
            .map_err(|e| anyhow!(fail_message(&format!("Failed to launch rsync: {}", e))))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            upload_spinner.stop_and_persist(&fail_symbol(), fail_message("Upload failed."));
            mark_failed(
                &deploy_ref,
                &created_deployment,
                &config,
                env,
                &access_token,
            )
            .await;
            return Err(anyhow!(fail_message(&format!(
                "rsync of '{}' failed (status {}): {}",
                transfer.local_rel,
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ))));
        }
    }

    // known_hosts_file must stay alive until all rsync calls complete.
    drop(known_hosts_file);

    upload_spinner.stop_and_persist(&succeed_symbol(), succeed_message("Upload complete."));

    // ── Step 8: SSH remote restart ───────────────────────────────────────────
    //
    // Runs the deployed app via pm2 inside the standalone directory.
    //
    // We always delete the existing pm2 process (if any) and start fresh. A
    // bare `pm2 restart` would re-execute the old command (e.g. `next start
    // --port XXXX` from a previous git-push deploy), which fails when the
    // working directory now contains standalone output instead of the full
    // Next.js build tree.
    //
    // If the server has an operator-managed ecosystem config (.cjs or .js),
    // prefer it as the source of truth for runtime env and pm2 settings. Otherwise fall back
    // to `node server.js` with the minimal inline env needed to bind the app.
    //
    // The port defaults to 3000 and can be overridden with `port = XXXX` in
    // .smb/config.toml — it must match the nginx upstream configuration.

    let port = config.project.port.unwrap_or(3000);
    let runtime_subdir_for_shell = runtime_subdir.clone().unwrap_or_default();

    // Build the ecosystem.config.cjs content from server-side pm2_env, if available.
    // Injected into the deploy script and written only when no config file exists yet.
    let ecosystem_config_content = {
        let mut env_entries = format!(
            r#"        NODE_ENV: "production",
        PORT: {port},
        HOSTNAME: "127.0.0.1",
"#
        );
        if let Some(pm2_env) = &config.project.pm2_env {
            for (key, value) in pm2_env {
                // Skip keys already emitted above
                if key == "NODE_ENV" || key == "PORT" || key == "HOSTNAME" {
                    continue;
                }
                let val_str = match value {
                    serde_json::Value::String(s) => {
                        let escaped = s
                            .replace('\\', "\\\\")
                            .replace('"', "\\\"")
                            .replace('\n', "\\n")
                            .replace('\r', "\\r");
                        format!(r#""{escaped}""#)
                    }
                    other => other.to_string(),
                };
                env_entries.push_str(&format!("        {key}: {val_str},\n"));
            }
        }
        format!(
            r#"module.exports = {{
  apps: [
    {{
      name: "{pm2_app}",
      script: "server.js",
      cwd: "",  // filled at runtime via $APP_PATH
      env_production: {{
{env_entries}      }},
    }},
  ],
}};
"#
        )
    };

    let deploy_script = format!(
        r#"set -e
    APP_PATH="{remote_path}"
    PM2_APP="{pm2_app}"
    RUNTIME_SUBDIR="{runtime_subdir}"

    case "$APP_PATH" in
        /*) ;;
        *) APP_PATH="$HOME/$APP_PATH" ;;
    esac

    if [ ! -d "$APP_PATH" ]; then
        echo "Error: $APP_PATH is not a directory."
        exit 1
    fi

    cd "$APP_PATH"
    mkdir -p logs

    # Migrate legacy ecosystem.config.js to .cjs so it works when
    # package.json has "type": "module".
    if [ -f ecosystem.config.js ] && [ ! -f ecosystem.config.cjs ]; then
        mv ecosystem.config.js ecosystem.config.cjs
    fi

    # Write ecosystem.config.cjs from smbCloud server config if none exists yet.
    if [ ! -f ecosystem.config.cjs ] && [ ! -f ecosystem.config.js ]; then
        cat > ecosystem.config.cjs << 'ECOSYSTEM_EOF'
{ecosystem_config}
ECOSYSTEM_EOF
        # Patch the cwd field to the actual runtime path
        node --input-type=commonjs -e "
          var fs = require('fs');
          var src = fs.readFileSync('ecosystem.config.cjs', 'utf8');
          src = src.replace('cwd: \"\"', 'cwd: \"' + process.argv[1] + '\"');
          fs.writeFileSync('ecosystem.config.cjs', src);
        " "$APP_PATH" 2>/dev/null || true
    fi

    # The monorepo root package.json may declare "type": "module", which
    # Next.js copies into .next/standalone/. Standalone server.js and our
    # shims use require() (CJS). Strip the field so Node treats .js as CJS.
    if [ -f package.json ] && grep -q '"type"' package.json; then
        node --input-type=commonjs -e "
          var fs = require('fs');
          var p = JSON.parse(fs.readFileSync('package.json','utf8'));
          delete p.type;
          fs.writeFileSync('package.json', JSON.stringify(p,null,2)+'\\n');
        " 2>/dev/null || sed -i '"'"'"type".*"module"'"'d' package.json
    fi

    # pnpm standalone output buries peer deps inside node_modules/.pnpm/
    # without the top-level symlinks Node needs for require.resolve().
    # First mirror pnpm's own virtual node_modules directory when it exists —
    # this preserves the exact version selection pnpm already resolved, which is
    # especially important for scoped packages like @swc/helpers.
    # Then fall back to scanning individual .pnpm store entries for anything the
    # virtual directory did not expose.
    if [ -d "node_modules/.pnpm/node_modules" ]; then
        for pkg_path in node_modules/.pnpm/node_modules/*; do
            [ -e "$pkg_path" ] || continue
            pkg_name=$(basename "$pkg_path")
            if [ "${{pkg_name:0:1}}" = "@" ] && [ -d "$pkg_path" ]; then
                mkdir -p "node_modules/$pkg_name"
                for sub_path in "$pkg_path"/*; do
                    [ -e "$sub_path" ] || continue
                    sub_name=$(basename "$sub_path")
                    rm -rf "node_modules/$pkg_name/$sub_name"
                    ln -sfn "$APP_PATH/$sub_path" "node_modules/$pkg_name/$sub_name"
                done
            else
                rm -rf "node_modules/$pkg_name"
                ln -sfn "$APP_PATH/$pkg_path" "node_modules/$pkg_name"
            fi
        done
    fi

    if [ -d "node_modules/.pnpm" ]; then
        find node_modules/.pnpm -mindepth 2 -maxdepth 2 -type d -name "node_modules" 2>/dev/null | while read pnpm_nm; do
            for pkg_path in "$pnpm_nm"/*; do
                [ -e "$pkg_path" ] || continue
                pkg_name=$(basename "$pkg_path")
                if [ "${{pkg_name:0:1}}" = "@" ] && [ -d "$pkg_path" ]; then
                    mkdir -p "node_modules/$pkg_name"
                    for sub_path in "$pkg_path"/*; do
                        [ -e "$sub_path" ] || continue
                        sub_name=$(basename "$sub_path")
                        # Count files in this .pnpm store entry.
                        entry_files=$(find "$sub_path" -maxdepth 2 -type f 2>/dev/null | wc -l | tr -d ' ')
                        existing="node_modules/$pkg_name/$sub_name"
                        if [ -e "$existing" ]; then
                            # Only replace if existing has fewer files than the .pnpm store entry.
                            existing_files=$(find "$existing" -maxdepth 2 -type f 2>/dev/null | wc -l | tr -d ' ')
                            if [ "$entry_files" -gt "$existing_files" ]; then
                                rm -rf "$existing"
                                ln -sfn "$APP_PATH/$sub_path" "$existing"
                            fi
                        else
                            ln -sfn "$APP_PATH/$sub_path" "$existing"
                        fi
                    done
                else
                    if [ -e "node_modules/$pkg_name" ]; then
                        continue
                    fi
                    ln -sfn "$APP_PATH/$pkg_path" "node_modules/$pkg_name"
                fi
            done
        done
    fi

    if [ -n "$RUNTIME_SUBDIR" ]; then
        cat > server.js <<EOF
import("./$RUNTIME_SUBDIR/server.js").catch(function(e){{console.error(e);process.exit(1)}})
EOF
    fi

    # Backward compatibility: older operator-managed ecosystem config files
    # may still point PM2 at `server.js` or `.next/standalone/server.js` in the
    # app root even when standalone preserves the source path inside `web/`.
    mkdir -p .next/standalone
    rm -rf .next/standalone/node_modules
    ln -sfn ../../node_modules .next/standalone/node_modules

    if [ -n "$RUNTIME_SUBDIR" ]; then
        cat > .next/standalone/server.js <<EOF
import("../../$RUNTIME_SUBDIR/server.js").catch(function(e){{console.error(e);process.exit(1)}})
EOF
    else
        ln -sfn ../../server.js .next/standalone/server.js
    fi

    echo "Starting $PM2_APP with pm2..."
    if pm2 describe "$PM2_APP" > /dev/null 2>&1; then
        pm2 delete "$PM2_APP"
    fi

    if [ -f ecosystem.config.cjs ]; then
        pm2 start ecosystem.config.cjs --only "$PM2_APP" --env production
    elif [ -f ecosystem.config.js ]; then
        pm2 start ecosystem.config.js --only "$PM2_APP" --env production
    else
        NODE_ENV=production PORT={port} HOSTNAME=127.0.0.1 pm2 start node --name "$PM2_APP" -- server.js
    fi

    pm2 save
    echo "Done."
    "#,
        remote_path = remote_path,
        pm2_app = pm2_app,
        runtime_subdir = runtime_subdir_for_shell,
        port = port,
        ecosystem_config = ecosystem_config_content,
    );

    let mut restart_spinner = Spinner::new(
        Spinners::SimpleDotsScrolling,
        succeed_message(&format!("Restarting {} on server…", pm2_app)),
    );

    // Fresh known_hosts file for the SSH exec (previous one was dropped).
    let mut ssh_known_hosts_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp known_hosts file: {}", e))?;
    writeln!(
        ssh_known_hosts_file,
        "{}",
        known_hosts::for_host(&rsync_host)
    )
    .map_err(|e| anyhow!("Failed to write known_hosts: {}", e))?;

    let mut child = Command::new("ssh")
        .args([
            "-i",
            &identity_file_str,
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            &format!(
                "UserKnownHostsFile={}",
                ssh_known_hosts_file.path().display()
            ),
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "PasswordAuthentication=no",
            "-o",
            "BatchMode=yes",
            &format!("git@{}", rsync_host),
            "bash -s",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!(fail_message(&format!("Failed to spawn SSH: {}", e))))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(deploy_script.as_bytes())
            .map_err(|e| anyhow!("Failed to write deploy script to SSH stdin: {}", e))?;
    }

    // stdin was dropped at the end of the if-let block above, sending EOF to
    // remote bash. wait_with_output() reads stdout/stderr then waits for exit.
    let ssh_output = child
        .wait_with_output()
        .map_err(|e| anyhow!("Failed to wait for SSH process: {}", e))?;

    drop(ssh_known_hosts_file);

    if !ssh_output.status.success() {
        restart_spinner.stop_and_persist(&fail_symbol(), fail_message("Remote restart failed."));
        let stderr = String::from_utf8_lossy(&ssh_output.stderr);
        let stdout = String::from_utf8_lossy(&ssh_output.stdout);
        let details = if !stderr.trim().is_empty() {
            stderr
        } else {
            stdout
        };
        if !details.trim().is_empty() {
            eprintln!("{}", details.trim());
        }
        mark_failed(
            &deploy_ref,
            &created_deployment,
            &config,
            env,
            &access_token,
        )
        .await;
        return Err(anyhow!(fail_message(&format!(
            "SSH deploy script exited with status {}",
            ssh_output.status
        ))));
    }

    restart_spinner.stop_and_persist(
        &succeed_symbol(),
        succeed_message(&format!("{} restarted.", pm2_app)),
    );

    // ── Step 9: mark deployment as Done ──────────────────────────────────────

    if let Some(ref deployment) = created_deployment {
        match update(
            env,
            client(),
            access_token,
            config.project.id,
            deployment.id,
            DeploymentPayload {
                commit_hash: deploy_ref,
                status: DeploymentStatus::Done,
                frontend_app_id: config.project.frontend_app_id.clone(),
            },
        )
        .await
        {
            Ok(_) => println!("App is running {}", succeed_symbol()),
            Err(e) => eprintln!("Error updating deployment status to Done: {}", e),
        }
    }

    Ok(CommandResult {
        spinner: Spinner::new(Spinners::Hamburger, String::new()),
        symbol: succeed_symbol(),
        msg: succeed_message("Deployment complete."),
    })
}

/// Mark the in-flight deployment record as Failed.
/// Called on any early-return error path so the dashboard reflects reality.
async fn mark_failed(
    deploy_ref: &str,
    created_deployment: &Option<smbcloud_model::project::Deployment>,
    config: &Config,
    env: Environment,
    access_token: &str,
) {
    if let Some(ref deployment) = created_deployment {
        let _ = update(
            env,
            client(),
            access_token.to_owned(),
            config.project.id,
            deployment.id,
            DeploymentPayload {
                commit_hash: deploy_ref.to_owned(),
                status: DeploymentStatus::Failed,
                frontend_app_id: config.project.frontend_app_id.clone(),
            },
        )
        .await;
    }
}
