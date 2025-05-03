use anyhow::{anyhow, Result};
use console::style;
use log::debug;
use regex::Regex;
use reqwest::{Client, Response, StatusCode};
use smbcloud_model::account::SmbAuthorization;
use smbcloud_networking::{
    constants::{
        GH_OAUTH_CLIENT_ID, GH_OAUTH_REDIRECT_HOST, GH_OAUTH_REDIRECT_PORT, PATH_AUTHORIZE,
    },
    environment::Environment,
    smb_base_url_builder, smb_token_file_path,
};
use spinners::Spinner;
use std::{
    fs::{create_dir_all, OpenOptions},
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
};
use url_builder::URLBuilder;

use crate::ui::fail_message;

pub async fn authorize_github(env: &Environment) -> Result<SmbAuthorization> {
    // Spin up a simple localhost server to listen for the GitHub OAuth callback
    // setup_oauth_callback_server();
    // Open the GitHub OAuth URL in the user's browser
    let mut spinner = Spinner::new(
        spinners::Spinners::BouncingBall,
        style("🚀 Getting your GitHub information...")
            .green()
            .bold()
            .to_string(),
    );

    let rx = match open::that(build_github_oauth_url()) {
        Ok(_) => {
            let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
            debug!(
                "Setting up OAuth callback server... (tx: {:#?}, rx: {:#?})",
                &tx, &rx
            );
            tokio::spawn(async move {
                setup_oauth_callback_server(tx);
            });
            rx
        }
        Err(_) => {
            let error = anyhow!("Failed to open a browser.");
            return Err(error);
        }
    };

    spinner.stop_and_persist("⌛", "Waiting for the authorization.".into());

    debug!("Waiting for code from channel...");

    match rx.recv() {
        Ok(code) => {
            debug!("Got code from channel: {:#?}", &code);
            //Err(anyhow!("Failed to get code from channel."))
            process_connect_github(*env, code).await
        }
        Err(e) => {
            let error = anyhow!("Failed to get code from channel: {e}");
            Err(error)
        }
    }
}

fn setup_oauth_callback_server(tx: Sender<String>) {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", GH_OAUTH_REDIRECT_PORT)).unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream, tx.clone());
    }
}

fn handle_connection(mut stream: TcpStream, tx: Sender<String>) {
    let buf_reader = BufReader::new(&stream);
    let request_line = &buf_reader.lines().next().unwrap().unwrap();

    debug!("Request: {:#?}", request_line);

    let code_regex = Regex::new(r"code=([^&]*)").unwrap();

    let (status_line, contents) = match code_regex.captures(request_line) {
        Some(group) => {
            let code = group.get(1).unwrap().as_str();
            debug!("Code: {:#?}", code);
            debug!("Sending code to channel...");
            debug!("Channel: {:#?}", &tx);
            match tx.send(code.to_string()) {
                Ok(_) => {
                    debug!("Code sent to channel.");
                }
                Err(e) => {
                    debug!("Failed to send code to channel: {e}");
                }
            }
            (
                "HTTP/1.1 200 OK",
                "<!DOCTYPE html>

                <head>
                    <meta charset='utf-8'>
                    <title>Hello!</title>
                </head>
                
                <body>
                    <h1>Authenticated!</h1>
                    <p>Back to the terminal console to finish your registration.</p>
                </body>",
            )
        }
        None => {
            debug!("Code not found.");
            (
                "HTTP/1.1 404 NOT FOUND",
                "<!DOCTYPE html>
                <html lang='en'>
                
                <head>
                    <meta charset='utf-8'>
                    <title>404 Not found</title>
                </head>
                
                <body>
                    <h1>Oops!</h1>
                    <p>Sorry, I don't know what you're asking for.</p>
                </body>
                
                </html>",
            )
        }
    };

    debug!("Contents: {:#?}", &contents);
    let response = format!("{status_line}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

// Get access token
pub async fn process_connect_github(env: Environment, code: String) -> Result<SmbAuthorization> {
    let response = Client::new()
        .post(build_authorize_smb_url(env))
        .body(format!("gh_code={}", code))
        .header("Accept", "application/json")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;
    let mut spinner = Spinner::new(
        spinners::Spinners::BouncingBall,
        style("🚀 Authorizing your account...")
            .green()
            .bold()
            .to_string(),
    );
    // println!("Response: {:#?}", &response);
    match response.status() {
        StatusCode::OK => {
            // Account authorized and token received
            spinner.stop_and_persist("✅", "You are logged in with your GitHub account!".into());
            save_token(env, &response).await?;
            let result = response.json().await?;
            // println!("Result: {:#?}", &result);
            Ok(result)
        }
        StatusCode::NOT_FOUND => {
            // Account not found and we show signup option
            spinner.stop_and_persist("🥲", "Account not found. Please signup!".into());
            let result = response.json().await?;
            // println!("Result: {:#?}", &result);
            Ok(result)
        }
        StatusCode::UNPROCESSABLE_ENTITY => {
            // Account found but email not verified
            spinner.stop_and_persist("🥹", "Unverified email!".into());
            let result = response.json().await?;
            // println!("Result: {:#?}", &result);
            Ok(result)
        }
        _ => {
            // Other errors
            let error = anyhow!("Error while authorizing with GitHub.");
            Err(error)
        }
    }
}

fn build_authorize_smb_url(env: Environment) -> String {
    let mut url_builder = smb_base_url_builder(env);
    url_builder.add_route(PATH_AUTHORIZE);
    url_builder.build()
}

fn build_github_oauth_url() -> String {
    let mut url_builder = github_base_url_builder();
    url_builder
        .add_route("login/oauth/authorize")
        .add_param("scope", "user")
        .add_param("state", "smbcloud");
    url_builder.build()
}

fn github_base_url_builder() -> URLBuilder {
    let redirect_url = format!("{}:{}", GH_OAUTH_REDIRECT_HOST, GH_OAUTH_REDIRECT_PORT);

    let mut url_builder = URLBuilder::new();
    url_builder
        .set_protocol("https")
        .set_host("github.com")
        .add_param("client_id", GH_OAUTH_CLIENT_ID)
        .add_param("redirect_uri", &redirect_url);
    url_builder
}

pub async fn save_token(env: Environment, response: &Response) -> Result<()> {
    let headers = response.headers();
    // println!("Headers: {:#?}", &headers);
    match headers.get("Authorization") {
        Some(token) => {
            debug!("{}", token.to_str()?);
            match home::home_dir() {
                Some(path) => {
                    debug!("{}", path.to_str().unwrap());
                    create_dir_all(path.join(env.smb_dir()))?;
                    let mut file = OpenOptions::new()
                        .create(true)
                        .truncate(true)
                        .write(true)
                        .open([path.to_str().unwrap(), "/", &env.smb_dir(), "/token"].join(""))?;
                    file.write_all(token.to_str()?.as_bytes())?;
                    Ok(())
                }
                None => Err(anyhow!("Failed to get home directory.")),
            }
        }
        None => Err(anyhow!("Failed to get token. Probably a backend issue.")),
    }
}

pub async fn protected_request(env: Environment) -> Result<()> {
    // Check if token file exists
    if smb_token_file_path(env).is_none() {
        return Err(anyhow!(
            fail_message("Please authorize your account first with `smb account login` command.")
        ));
    }
    Ok(())
}