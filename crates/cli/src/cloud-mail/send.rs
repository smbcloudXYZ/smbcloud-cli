//! Shared plumbing for outbound transactional email.
//!
//! Sending is the one Mail operation that does not use the session token from
//! `smb login`. The send API authenticates with a Mail app API key (`smb_mail_…`)
//! that is scoped to that app's verified sending domain, so the key has to come
//! from the caller: `--api-key`, or `SMB_MAIL_API_KEY` in the environment (which
//! is also how an MCP client passes it through its server config).
//!
//! Both `smb mail send` and the `mail_send` MCP tool build their message here so
//! the validation and the error wording stay in one place.

use {
    anyhow::{anyhow, Result},
    smbcloud_email_sdk::{EmailClient, EmailCredentials, EmailMessage, Environment, SendEmail},
};

/// Environment variable holding the Mail app API key.
pub const API_KEY_ENV: &str = "SMB_MAIL_API_KEY";

/// One outbound message, in the shape both front-ends collect it.
#[derive(Debug, Default)]
pub struct OutboundEmail {
    pub from: String,
    pub to: Vec<String>,
    pub subject: Option<String>,
    pub html: Option<String>,
    pub text: Option<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Vec<String>,
    pub idempotency_key: Option<String>,
}

/// Resolve the API key: the explicit value first, then the environment.
///
/// `smb mail send` already reads the variable through clap, so this mostly
/// serves the MCP tool, which has no flags to read.
pub fn resolve_api_key(explicit: Option<String>) -> Result<String> {
    let key = explicit
        .filter(|key| !key.trim().is_empty())
        .or_else(|| std::env::var(API_KEY_ENV).ok())
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());

    key.ok_or_else(|| {
        anyhow!(
            "No Mail app API key. Set {API_KEY_ENV}, or pass --api-key from the CLI. \
             Mint a key for your Mail app in the smbCloud console; MCP clients put it \
             in the server's env block."
        )
    })
}

/// Send the message, mapping validation and API failures to plain errors.
pub async fn send_email(
    environment: Environment,
    api_key: &str,
    email: OutboundEmail,
) -> Result<EmailMessage> {
    let message = build_message(email)?;
    let client = EmailClient::from_credentials(environment, EmailCredentials { api_key });

    client
        .send(&message)
        .await
        .map_err(|error| anyhow!("Failed to send email: {error}"))
}

/// Validate the parts the API requires before spending a round trip on them.
fn build_message(email: OutboundEmail) -> Result<SendEmail> {
    let from = email.from.trim().to_string();
    if from.is_empty() {
        return Err(anyhow!("A sender address is required."));
    }

    let to = non_empty(email.to);
    if to.is_empty() {
        return Err(anyhow!("At least one recipient is required."));
    }

    let html = non_empty_string(email.html);
    let text = non_empty_string(email.text);
    if html.is_none() && text.is_none() {
        return Err(anyhow!(
            "An email needs an HTML body, a text body, or both."
        ));
    }

    let mut message = SendEmail::new(from, to);

    if let Some(subject) = non_empty_string(email.subject) {
        message = message.subject(subject);
    }
    if let Some(html) = html {
        message = message.html(html);
    }
    if let Some(text) = text {
        message = message.text(text);
    }

    let cc = non_empty(email.cc);
    if !cc.is_empty() {
        message = message.cc(cc);
    }
    let bcc = non_empty(email.bcc);
    if !bcc.is_empty() {
        message = message.bcc(bcc);
    }
    let reply_to = non_empty(email.reply_to);
    if !reply_to.is_empty() {
        message = message.reply_to(reply_to);
    }

    if let Some(key) = non_empty_string(email.idempotency_key) {
        message = message.idempotency_key(key);
    }

    Ok(message)
}

fn non_empty(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn non_empty_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> OutboundEmail {
        OutboundEmail {
            from: "billing@example.com".to_string(),
            to: vec!["customer@acme.com".to_string()],
            text: Some("Thanks".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn builds_a_minimal_message() {
        let message = build_message(minimal()).expect("valid message");
        assert_eq!(message.from, "billing@example.com");
        assert_eq!(message.to, vec!["customer@acme.com".to_string()]);
    }

    #[test]
    fn rejects_a_message_with_no_body() {
        let email = OutboundEmail {
            text: None,
            ..minimal()
        };
        assert!(build_message(email).is_err());
    }

    #[test]
    fn rejects_blank_recipients() {
        let email = OutboundEmail {
            to: vec!["   ".to_string()],
            ..minimal()
        };
        assert!(build_message(email).is_err());
    }

    #[test]
    fn resolve_api_key_prefers_the_explicit_value() {
        let key = resolve_api_key(Some("smb_mail_explicit".to_string())).expect("a key");
        assert_eq!(key, "smb_mail_explicit");
    }

    #[test]
    fn resolve_api_key_rejects_a_blank_value() {
        // With no environment fallback set in this process, a blank explicit
        // value has to fail rather than send with an empty Authorization header.
        if std::env::var(API_KEY_ENV).is_ok() {
            return;
        }
        assert!(resolve_api_key(Some("  ".to_string())).is_err());
    }
}
