use thiserror::Error;

#[derive(Debug, Error)]
pub enum GresiqError {
    /// The HTTP layer failed before we even got a response — network down,
    /// DNS failure, TLS handshake, that sort of thing.
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The gateway replied with a non-2xx status. The message is whatever
    /// the server put in the response body, or the HTTP reason phrase if
    /// the body wasn't readable.
    #[error("GresIQ API error {status}: {message}")]
    Api { status: u16, message: String },
}
