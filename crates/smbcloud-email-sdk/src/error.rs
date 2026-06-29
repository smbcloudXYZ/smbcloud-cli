use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    /// The HTTP layer failed before we got a response — network down, DNS
    /// failure, TLS handshake, that sort of thing.
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The API replied with a non-2xx status. `message` is whatever the server
    /// put in the response body (typically `{ "message": "..." }`), or the HTTP
    /// reason phrase if the body wasn't readable.
    #[error("email API error {status}: {message}")]
    Api { status: u16, message: String },
}
