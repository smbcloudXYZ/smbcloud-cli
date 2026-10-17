use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use xcrs::{
    AndroidDebugBridge, AndroidKit, HostCropRequest, HostInspect, HostInspectRect,
    HostInspectScale, HostOcrRequest, HostProbeAxis, HostProbeRequest, IosAppTest,
    XcodeCommandLineTools,
};

#[derive(Debug, Parser)]
#[command(name = "xcrs")]
#[command(version)]
#[command(about = "Cross-platform mobile and TV app automation")]
struct Cli {
    /// Run as a standalone Model Context Protocol server over stdio.
    #[arg(long)]
    mcp: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(subcommand)]
    Simulator(SimulatorCommand),
    #[command(subcommand)]
    AndroidKit(AndroidKitCommand),
    /// Inspect the macOS host: on-screen windows, image text, crops, and pixel probes.
    #[command(subcommand)]
    Host(HostCommand),
    IosAppTest {
        #[arg(long, conflicts_with = "simulator_udid")]
        simulator_name: Option<String>,
        #[arg(long, conflicts_with = "simulator_name")]
        simulator_udid: Option<String>,
        #[arg(long)]
        app_path: PathBuf,
        #[arg(long)]
        bundle_id: String,
        #[arg(long)]
        terminate_before_launch: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
enum AndroidKitCommand {
    Install {
        #[arg(long)]
        serial: String,
        #[arg(long)]
        apk: PathBuf,
    },
    Start {
        #[arg(long)]
        serial: String,
    },
    Status {
        #[arg(long)]
        serial: String,
        #[arg(long)]
        json: bool,
    },
    Stop {
        #[arg(long)]
        serial: String,
    },
}

#[derive(Debug, Subcommand)]
enum HostCommand {
    /// List the host's on-screen windows, ordered by layer and then id.
    Windows {
        /// Only windows owned by this application name.
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Read the text in an image with the platform's Vision text recognizer.
    Ocr {
        #[arg(long)]
        image: PathBuf,
        /// Recognition language, repeatable; omit for the platform's own default.
        #[arg(long = "language")]
        languages: Vec<String>,
        /// Pixels per caller unit, e.g. 2 to pass coordinates in Retina points.
        #[arg(long, default_value_t = HostInspectScale::PIXELS, value_parser = parse_scale)]
        scale: HostInspectScale,
        #[arg(long)]
        json: bool,
    },
    /// Crop an image to a rect and write the result out as a PNG.
    Crop {
        #[arg(long)]
        image: PathBuf,
        /// Crop rect as x,y,width,height in caller units.
        #[arg(long, value_parser = parse_rect)]
        rect: HostInspectRect,
        /// Destination path for the cropped PNG.
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = HostInspectScale::PIXELS, value_parser = parse_scale)]
        scale: HostInspectScale,
        #[arg(long)]
        json: bool,
    },
    /// Read one line of an image as a run of colors.
    Probe {
        #[arg(long)]
        image: PathBuf,
        /// Probe a horizontal line: the row at --position.
        #[arg(long, conflicts_with = "column")]
        row: bool,
        /// Probe a vertical line: the column at --position.
        #[arg(long, conflicts_with = "row")]
        column: bool,
        /// Row or column to read, in caller units.
        #[arg(long)]
        position: f64,
        /// First unit of the probed line; defaults to the image edge.
        #[arg(long)]
        from: Option<f64>,
        /// Last unit of the probed line; defaults to the image edge.
        #[arg(long)]
        to: Option<f64>,
        #[arg(long, default_value_t = HostInspectScale::PIXELS, value_parser = parse_scale)]
        scale: HostInspectScale,
        #[arg(long)]
        json: bool,
    },
}

fn parse_scale(value: &str) -> Result<HostInspectScale> {
    value.parse()
}

fn parse_rect(value: &str) -> Result<HostInspectRect> {
    value.parse()
}

/// Resolve the probe direction. Clap has already rejected `--row --column`;
/// this covers the remaining case of neither flag being given.
fn probe_axis(row: bool, column: bool) -> Result<HostProbeAxis> {
    match (row, column) {
        (true, false) => Ok(HostProbeAxis::Row),
        (false, true) => Ok(HostProbeAxis::Column),
        _ => Err(anyhow!("probe needs exactly one of --row or --column")),
    }
}

#[derive(Debug, Subcommand)]
enum SimulatorCommand {
    Find {
        #[arg(long)]
        name: String,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.mcp {
        return xcrs::mcp::serve().await;
    }

    let command = cli
        .command
        .ok_or_else(|| anyhow!("a command is required unless --mcp is provided"))?;
    let tools = XcodeCommandLineTools::new();

    match command {
        Commands::AndroidKit(AndroidKitCommand::Install { serial, apk }) => {
            AndroidDebugBridge::new().install_androidkit(&serial, &apk)?;
            println!("Installed XCRS AndroidKit on {serial}");
        }
        Commands::AndroidKit(AndroidKitCommand::Start { serial }) => {
            AndroidDebugBridge::new().start_androidkit(&serial).await?;
            println!("Started XCRS AndroidKit on {serial}");
        }
        Commands::AndroidKit(AndroidKitCommand::Status { serial, json }) => {
            let info = AndroidKit::new()
                .call(serial.clone(), "device.info", serde_json::json!({}))
                .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("XCRS AndroidKit is ready on {serial}");
            }
        }
        Commands::AndroidKit(AndroidKitCommand::Stop { serial }) => {
            AndroidDebugBridge::new().stop_androidkit(&serial)?;
            println!("Stopped XCRS AndroidKit on {serial}");
        }
        Commands::Host(HostCommand::Windows { owner, json }) => {
            let windows = HostInspect::new().windows(owner.as_deref())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&windows)?);
            } else if windows.windows.is_empty() {
                println!("No on-screen windows");
            } else {
                for window in windows.windows {
                    println!(
                        "{} {} {} {}",
                        window.id, window.owner, window.title, window.bounds
                    );
                }
            }
        }
        Commands::Host(HostCommand::Ocr {
            image,
            languages,
            scale,
            json,
        }) => {
            let result = HostInspect::new().recognize_text(&HostOcrRequest {
                image,
                scale,
                languages,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.results.is_empty() {
                println!("No text found in {}", result.image.path);
            } else {
                for recognized in result.results {
                    println!("{:.2} {}", recognized.confidence, recognized.text);
                }
            }
        }
        Commands::Host(HostCommand::Crop {
            image,
            rect,
            out,
            scale,
            json,
        }) => {
            let result = HostInspect::new().crop(&HostCropRequest {
                image,
                rect,
                out,
                scale,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!(
                    "Wrote {} ({}x{})",
                    result.out, result.pixel_width, result.pixel_height
                );
            }
        }
        Commands::Host(HostCommand::Probe {
            image,
            row,
            column,
            position,
            from,
            to,
            scale,
            json,
        }) => {
            let axis = probe_axis(row, column)?;
            let result = HostInspect::new().probe(&HostProbeRequest {
                image,
                axis,
                position,
                from,
                to,
                scale,
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                for run in result.runs {
                    println!("{} {} {} {}", run.start, run.end, run.width, run.color);
                }
            }
        }
        Commands::Simulator(SimulatorCommand::Find { name, json }) => {
            let simulator = tools.simctl().find_simulator_by_name(&name)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&simulator)?);
            } else {
                println!("{} {}", simulator.name, simulator.udid);
            }
        }
        Commands::Simulator(SimulatorCommand::List { json }) => {
            let simulators = tools.simctl().list_simulators()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&simulators)?);
            } else {
                for simulator in simulators {
                    println!("{} {} {}", simulator.name, simulator.udid, simulator.state);
                }
            }
        }
        Commands::IosAppTest {
            simulator_name,
            simulator_udid,
            app_path,
            bundle_id,
            terminate_before_launch,
            json,
        } => {
            let result = tools.run_ios_app_test(&IosAppTest {
                simulator_name,
                simulator_udid,
                app_path,
                bundle_id,
                terminate_before_launch,
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!(
                    "Launched {} on {} ({})",
                    result.bundle_id, result.simulator.name, result.simulator.udid
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(arguments: &[&str]) -> HostCommand {
        match Cli::try_parse_from(arguments)
            .unwrap_or_else(|error| panic!("failed to parse {arguments:?}: {error}"))
            .command
        {
            Some(Commands::Host(command)) => command,
            other => panic!("expected a host command, got {other:?}"),
        }
    }

    #[test]
    fn parses_the_host_windows_command() {
        let HostCommand::Windows { owner, json } = host(&["xcrs", "host", "windows", "--json"])
        else {
            panic!("expected the windows command");
        };

        assert_eq!(owner, None);
        assert!(json);
    }

    #[test]
    fn filters_windows_by_owner() {
        let HostCommand::Windows { owner, json } =
            host(&["xcrs", "host", "windows", "--owner", "Xcode"])
        else {
            panic!("expected the windows command");
        };

        assert_eq!(owner.as_deref(), Some("Xcode"));
        assert!(!json);
    }

    #[test]
    fn defaults_the_host_scale_to_pixels() {
        let HostCommand::Ocr {
            languages, scale, ..
        } = host(&["xcrs", "host", "ocr", "--image", "shot.png"])
        else {
            panic!("expected the ocr command");
        };

        assert!(languages.is_empty());
        assert_eq!(scale, HostInspectScale::PIXELS);
    }

    #[test]
    fn collects_repeated_languages_and_the_scale() {
        let HostCommand::Ocr {
            image,
            languages,
            scale,
            ..
        } = host(&[
            "xcrs",
            "host",
            "ocr",
            "--image",
            "shot.png",
            "--language",
            "en-US",
            "--language",
            "sv-SE",
            "--scale",
            "2",
        ])
        else {
            panic!("expected the ocr command");
        };

        assert_eq!(image, PathBuf::from("shot.png"));
        assert_eq!(languages, ["en-US", "sv-SE"]);
        assert_eq!(scale.pixels_per_unit(), 2.0);
    }

    #[test]
    fn parses_the_host_crop_rect() {
        let HostCommand::Crop {
            image, rect, out, ..
        } = host(&[
            "xcrs",
            "host",
            "crop",
            "--image",
            "shot.png",
            "--rect",
            "1,2,30,40",
            "--out",
            "crop.png",
        ])
        else {
            panic!("expected the crop command");
        };

        assert_eq!(image, PathBuf::from("shot.png"));
        assert_eq!(rect, HostInspectRect::new(1.0, 2.0, 30.0, 40.0));
        assert_eq!(out, PathBuf::from("crop.png"));
    }

    #[test]
    fn rejects_a_crop_rect_that_is_not_four_numbers() {
        let error = Cli::try_parse_from([
            "xcrs", "host", "crop", "--image", "shot.png", "--rect", "1,2,3", "--out", "crop.png",
        ])
        .expect_err("a three part rect should not parse");

        assert!(error.to_string().contains("x,y,w,h"), "{error}");
    }

    #[test]
    fn parses_the_probe_bounds() {
        let HostCommand::Probe {
            image,
            row,
            column,
            position,
            from,
            to,
            scale,
            json,
        } = host(&[
            "xcrs",
            "host",
            "probe",
            "--image",
            "shot.png",
            "--row",
            "--position",
            "12",
            "--from",
            "4",
            "--to",
            "20",
        ])
        else {
            panic!("expected the probe command");
        };

        assert_eq!(image, PathBuf::from("shot.png"));
        assert_eq!(probe_axis(row, column).expect("row"), HostProbeAxis::Row);
        assert_eq!(position, 12.0);
        assert_eq!(from, Some(4.0));
        assert_eq!(to, Some(20.0));
        assert_eq!(scale, HostInspectScale::PIXELS);
        assert!(!json);
    }

    #[test]
    fn rejects_a_probe_with_both_axes() {
        let error = Cli::try_parse_from([
            "xcrs",
            "host",
            "probe",
            "--image",
            "shot.png",
            "--row",
            "--column",
            "--position",
            "12",
        ])
        .expect_err("--row and --column should conflict");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn rejects_a_probe_without_an_axis() {
        let error = probe_axis(false, false).expect_err("an axis is required");

        assert!(error.to_string().contains("--row or --column"), "{error}");
    }
}
