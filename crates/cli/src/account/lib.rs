use {
    crate::token::smb_token_file_path::smb_token_file_path,
    anyhow::{anyhow, Result},
    console::style,
    log::debug,
    regex::Regex,
    reqwest::{Client, Response, StatusCode},
    smbcloud_model::account::SmbAuthorization,
    smbcloud_network::environment::Environment,
    smbcloud_networking::{
        constants::{
            GH_OAUTH_CLIENT_ID, GH_OAUTH_REDIRECT_HOST, GH_OAUTH_REDIRECT_PORT, PATH_AUTHORIZE,
        },
        smb_base_url_builder,
        smb_client::SmbClient,
    },
    spinners::Spinner,
    std::{
        fs::{create_dir_all, OpenOptions},
        io::{BufRead, BufReader, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver, Sender},
    },
    url_builder::URLBuilder,
};

pub async fn authorize_github(env: &Environment) -> Result<SmbAuthorization> {
    // Listen for the GitHub OAuth callback on localhost, then open the browser.
    let mut spinner = Spinner::new(
        spinners::Spinners::BouncingBall,
        style("🚀 Getting your GitHub information...")
            .green()
            .bold()
            .to_string(),
    );

    // Bind before opening the browser so a busy port is reported to the user
    // instead of leaving them waiting on a callback that can never arrive.
    let listener = TcpListener::bind(format!("127.0.0.1:{}", GH_OAUTH_REDIRECT_PORT))
        .map_err(|e| anyhow!("Could not listen on port {GH_OAUTH_REDIRECT_PORT}: {e}"))?;

    let rx = match open::that(build_github_oauth_url()) {
        Ok(_) => {
            let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
            debug!(
                "Setting up OAuth callback server... (tx: {:#?}, rx: {:#?})",
                tx, rx
            );
            tokio::spawn(async move {
                setup_oauth_callback_server(listener, tx);
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
            debug!("Got code from channel: {:#?}", code);
            process_connect_github(*env, code).await
        }
        Err(e) => {
            let error = anyhow!("Failed to get code from channel: {e}");
            Err(error)
        }
    }
}

fn setup_oauth_callback_server(listener: TcpListener, tx: Sender<String>) {
    for stream in listener.incoming() {
        let result = stream
            .map_err(anyhow::Error::from)
            .and_then(|stream| handle_connection(stream, tx.clone()));
        if let Err(e) = result {
            debug!("OAuth callback connection failed: {e}");
        }
    }
}

fn handle_connection(mut stream: TcpStream, tx: Sender<String>) -> Result<()> {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader
        .lines()
        .next()
        .ok_or_else(|| anyhow!("OAuth callback connection closed without a request."))??;

    debug!("Request: {:#?}", request_line);

    let code_regex = Regex::new(r"code=([^&]*)")?;

    let code = code_regex
        .captures(&request_line)
        .and_then(|group| group.get(1));

    let (status_line, contents) = match code {
        Some(code) => {
            let code = code.as_str();
            debug!("Code: {:#?}", code);
            debug!("Sending code to channel...");
            debug!("Channel: {:#?}", tx);
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

    debug!("Contents: {:#?}", contents);
    let response = format!("{status_line}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

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
    match response.status() {
        StatusCode::OK => {
            spinner.stop_and_persist("✅", "You are logged in with your GitHub account!".into());
            save_token(env, &response).await?;
            let result = response.json().await?;
            // println!("Result: {:#?}", &result);
            Ok(result)
        }
        StatusCode::NOT_FOUND => {
            spinner.stop_and_persist("🥲", "Account not found. Please sign up.".into());
            let result = response.json().await?;
            Ok(result)
        }
        StatusCode::UNPROCESSABLE_ENTITY => {
            spinner.stop_and_persist("🥹", "Please verify your email address.".into());
            let result = response.json().await?;
            Ok(result)
        }
        _ => {
            let error = anyhow!("Error while authorizing with GitHub.");
            Err(error)
        }
    }
}

fn build_authorize_smb_url(env: Environment) -> String {
    let secret = "secret";
    let mut url_builder = smb_base_url_builder(env, (&SmbClient::Cli, secret));
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
    match headers.get("Authorization") {
        Some(token) => {
            debug!("{}", token.to_str()?);
            store_token(env, token.to_str()?.to_string()).await
        }
        None => Err(anyhow!(
            "Failed to get the access token from the server response."
        )),
    }
}

pub async fn store_token(env: Environment, token: String) -> Result<()> {
    match home::home_dir() {
        Some(path) => {
            debug!("{}", path.to_str().unwrap());
            create_dir_all(path.join(env.smb_dir()))?;
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open([path.to_str().unwrap(), "/", &env.smb_dir(), "/token"].join(""))?;
            file.write_all(token.as_bytes())?;
            Ok(())
        }
        None => Err(anyhow!("Failed to get home directory.")),
    }
}

pub fn is_logged_in(env: Environment) -> bool {
    // Check if token file exists
    smb_token_file_path(env).is_some()
}
