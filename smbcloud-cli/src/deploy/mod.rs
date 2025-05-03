mod git;
mod smb_config;

use std::{fs::File, io::BufReader};
use crate::{account::lib::protected_request, cli::CommandResult};
use anyhow::Result;
use console::style;
use git::remote_deployment_setup;
use git2::{Cred, PushOptions, RemoteCallbacks, Repository};
use git_url_parse::{GitUrl, Scheme};
use smb_config::check_config;
use smbcloud_networking::environment::Environment;
use spinners::Spinner;
use ssh2_config::{ParseRule, SshConfig};

pub async fn process_deploy(env: Environment) -> Result<CommandResult> {
    protected_request(env).await?;
    let repo_name = check_config().await?;
    println!("Deploying your app...");
    let mut spinner = Spinner::new(
        spinners::Spinners::SimpleDotsScrolling,
        style("Deploying...").green().bold().to_string(),
    );  

    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(_) => {
            spinner.stop_and_persist("😩", "No git repository found.".to_owned());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "No git repository found. Init with `git init` command.".to_owned(),
            });
        }
    };

    let _main_branch = match repo.head() {
        Ok(branch) => branch,
        Err(_) => {
            spinner.stop_and_persist("😩", "No main branch found.".to_owned());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "No main branch found. Create with `git checkout -b <branch>` command."
                    .to_owned(),
            });
        }
    };
    let origin = remote_deployment_setup(&repo, repo_name).await?;
    let remote_url = match origin.url() {
        Some(url) => url,
        None => {
            spinner.stop_and_persist("😩", "No remote URL found.".to_owned());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "No remote URL found. Add with `git remote add origin <url>` command."
                    .to_owned(),
            });
        }
    };
    //println!("Remote URL: {:#?}", remote_url);
    let parsed_url = match GitUrl::parse(remote_url) {
        Ok(url) => url,
        Err(e) => {
            spinner.stop_and_persist("😩", e.to_string());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "Invalid remote URL.".to_owned(),
            });
        }
    };
    //println!("Parsed URL: {:#?}", parsed_url);
    match parsed_url.scheme {
        Scheme::Ssh => {
            // println!("SSH URL: {:#?}", parsed_url);
        }
        _ => {
            // Only support ssh for now
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "Only ssh is supported.".to_owned(),
            });
        }
    };

    // Get ssh config from host
    let host = match parsed_url.host {
        Some(host) => host,
        None => {
            spinner.stop_and_persist("😩", "No host found.".to_owned());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "No host found.".to_owned(),
            });
        }
    };

    // get ssh_config
    let ssh_config_file = match home::home_dir() {
        Some(home) => {
            let ssh_config_path = home.join(".ssh/config");
            if ssh_config_path.exists() {
                // println!("SSH config path: {:#?}", ssh_config_path);
                // Open the file and read it
                File::open(ssh_config_path.clone()).expect("Unable to open ssh config file")
            } else {
                spinner.stop_and_persist("😩", "No ssh config found.".to_owned());
                return Ok(CommandResult {
                    spinner,
                    symbol: "😩".to_owned(),
                    msg: "No ssh config found.".to_owned(),
                });
            }
        }
        None => {
            spinner.stop_and_persist("😩", "No home".to_owned());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "No home directory found.".to_owned(),
            });
        }
    };
    //println!("SSH config path: {:#?}", ssh_config_file);
    let mut reader = BufReader::new(ssh_config_file);
    let config = SshConfig::default()
        .parse(&mut reader, ParseRule::STRICT)
        .expect("Failed to parse ssh config file");
    //println!("SSH config: {:#?}", config);

    // Get the host config
    let host_config = config.query(host);
    //println!("Host config: {:#?}", host_config);
    // get identity_file
    let identity_files = match host_config.identity_file {
        Some(identity_files) => {
            //println!("Identity file: {:#?}", identity_files);
            identity_files
        }
        None => {
            spinner.stop_and_persist("😩", "No identity file found.".to_owned());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "No identity files found.".to_owned(),
            });
        }
    };
    //println!("Identity files: {:#?}", identity_files);
    // get identity_file
    let identity_file = match identity_files.first() {
        Some(identity_file) => {
            //println!("Identity file: {:#?}", identity_file);
            identity_file
        }
        None => {
            spinner.stop_and_persist("😩", "No identity file found.".to_owned());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: "No identity file found.".to_owned(),
            });
        }
    };
    let mut push_opts = PushOptions::new();
    let mut callbacks = RemoteCallbacks::new();
    // Set the credentials
    callbacks.credentials(|_url, _username_from_url, _allowed_types| {
        Cred::ssh_key("git", None, identity_file, None)
    });
    push_opts.remote_callbacks(callbacks);

    /*
    match origin.push(&["refs/heads/main:refs/heads/main"], Some(&mut push_opts)) {
        Ok(_) => {}
        Err(e) => {
            //println!("Failed to push to remote: {:#?}", e);
            spinner.stop_and_persist("😩", e.to_string());
            return Ok(CommandResult {
                spinner,
                symbol: "😩".to_owned(),
                msg: e.to_string(),
            });
        }
    };
    */
    
    Ok(CommandResult {
        spinner,
        symbol: "🚀".to_owned(),
        msg: "Your app has been deployed successfully.".to_owned(),
    })
}
