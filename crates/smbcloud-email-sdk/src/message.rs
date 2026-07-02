use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Delivery status of a transactional email, mirroring the server-side
/// integer-backed enum. Deserialized with `serde_repr` from the plain integer
/// the API returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum EmailStatus {
    Queued = 0,
    Sent = 1,
    Delivered = 2,
    Bounced = 3,
    Complained = 4,
    Failed = 5,
}

/// One attachment on an outbound email. `content_base64` is the file content
/// encoded as standard base64; the server reassembles it into the MIME message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_base64: String,
}

/// An outbound transactional email.
///
/// Build one with [`SendEmail::new`] and the builder helpers, then hand it to
/// [`crate::EmailClient::send`]. At least one of `html` or `text` is required;
/// `from` must be on the API key's verified domain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SendEmail {
    pub from: String,
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bcc: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reply_to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub tags: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

impl SendEmail {
    /// Start a message from a sender address and one or more recipients.
    pub fn new(from: impl Into<String>, to: impl IntoIterator<Item = impl Into<String>>) -> Self {
        SendEmail {
            from: from.into(),
            to: to.into_iter().map(Into::into).collect(),
            ..Default::default()
        }
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn cc(mut self, cc: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.cc = cc.into_iter().map(Into::into).collect();
        self
    }

    pub fn bcc(mut self, bcc: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.bcc = bcc.into_iter().map(Into::into).collect();
        self
    }

    pub fn reply_to(mut self, reply_to: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.reply_to = reply_to.into_iter().map(Into::into).collect();
        self
    }

    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Make a retried send safe: the same key returns the original message
    /// instead of sending again.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

/// One delivery event in a message's timeline (delivered, bounced, opened, …).
/// Present on [`EmailMessage`] only when fetched via
/// [`crate::EmailClient::get_message`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailEvent {
    pub id: String,
    pub event_type: u8,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub occurred_at: String,
}

/// A transactional email as returned by the API: the send result and, when
/// fetched by id, its delivery status and event timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub id: String,
    pub status: EmailStatus,
    pub provider_message_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub from_email: String,
    #[serde(default)]
    pub to_emails: Vec<String>,
    pub subject: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: Option<String>,
    /// Populated by `get_message`; empty for the bare send response.
    #[serde(default)]
    pub events: Vec<EmailEvent>,
}
