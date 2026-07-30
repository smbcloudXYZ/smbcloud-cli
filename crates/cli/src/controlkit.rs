use {
    crate::{
        cli::{CommandResult, ControlKitCommands},
        ui::{succeed_message, succeed_symbol},
    },
    anyhow::{anyhow, Context, Result},
    spinners::{Spinner, Spinners},
    std::{path::PathBuf, time::Duration},
    xcrs::{
        BuildControlKitDeviceRunner, ControlKit, StartControlKitDeviceRunner, XcodeCommandLineTools,
    },
};

pub async fn process_controlkit(command: ControlKitCommands) -> Result<CommandResult> {
    match command {
        ControlKitCommands::Build {
            project_path,
            scheme,
            configuration,
            device_udid,
            derived_data_path,
        } => {
            let test_run_path = XcodeCommandLineTools::new().build_controlkit_device_runner(
                &BuildControlKitDeviceRunner {
                    project_path,
                    scheme,
                    configuration,
                    device_udid,
                    derived_data_path,
                },
            )?;
            Ok(done_result(&format!(
                "Built ControlKit runner: {}",
                test_run_path.display()
            )))
        }
        ControlKitCommands::Start {
            device_udid,
            xctestrun_path,
            listen_host,
            listen_port,
            timeout_seconds,
            log_path,
        } => {
            let tools = XcodeCommandLineTools::new();
            let log_path = log_path.unwrap_or_else(|| {
                std::env::temp_dir().join(format!("controlkit-{device_udid}.log"))
            });
            let mut runner =
                tools.start_controlkit_device_runner(&StartControlKitDeviceRunner {
                    device_udid,
                    xctestrun_path,
                    listen_host,
                    listen_port,
                    log_path: log_path.clone(),
                })?;
            let controlkit = ControlKit::with_host(&runner.tunnel_host, runner.listen_port);
            let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_seconds);

            loop {
                if controlkit.health().await.is_ok() {
                    break;
                }
                if let Some(status) = runner.process.try_wait()? {
                    return Err(runner_failure(status.to_string(), &runner.log_path));
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(runner_failure(
                        format!("did not become healthy within {timeout_seconds} seconds"),
                        &runner.log_path,
                    ));
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            Ok(done_result(&format!(
                "ControlKit is running at http://[{}]:{} (PID {}, log {})",
                runner.tunnel_host,
                runner.listen_port,
                runner.process.id(),
                runner.log_path.display()
            )))
        }
        ControlKitCommands::Call {
            device_udid,
            host,
            port,
            method,
            params,
        } => {
            let host = match (host, device_udid) {
                (Some(host), None) => host,
                (None, Some(device_udid)) => {
                    XcodeCommandLineTools::new().device_tunnel_address(&device_udid)?
                }
                _ => {
                    return Err(anyhow!(
                        "provide either --host or --device-udid for the ControlKit runner"
                    ));
                }
            };
            let params = serde_json::from_str(&params)
                .with_context(|| format!("invalid JSON for --params: {params}"))?;
            let result = ControlKit::with_host(&host, port)
                .call(&method, params)
                .await?;
            Ok(done_result(&serde_json::to_string_pretty(&result)?))
        }
    }
}

fn runner_failure(reason: String, log_path: &PathBuf) -> anyhow::Error {
    let log = std::fs::read_to_string(log_path).unwrap_or_default();
    let tail = log.lines().rev().take(20).collect::<Vec<_>>();
    let tail = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
    if tail.is_empty() {
        anyhow!("ControlKit runner {reason}; see {}", log_path.display())
    } else {
        anyhow!("ControlKit runner {reason}:\n{tail}")
    }
}

fn done_result(message: &str) -> CommandResult {
    CommandResult {
        spinner: Spinner::new(Spinners::SimpleDotsScrolling, succeed_message("Done")),
        symbol: succeed_symbol(),
        msg: succeed_message(message),
    }
}
