use {
    chrono::{DateTime, Utc},
    serde::{Deserialize, Serialize},
    serde_repr::{Deserialize_repr, Serialize_repr},
    std::{collections::HashMap, fmt::Display},
};

#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MailStatus {
    #[default]
    PendingDns = 0,
    Active = 1,
    Suspended = 2,
}

impl Display for MailStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailStatus::PendingDns => write!(f, "pending_dns"),
            MailStatus::Active => write!(f, "active"),
            MailStatus::Suspended => write!(f, "suspended"),
        }
    }
}

#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum MailMessageStatus {
    #[default]
    Received = 0,
    Forwarded = 1,
    ForwardFailed = 2,
}

impl Display for MailMessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailMessageStatus::Received => write!(f, "received"),
            MailMessageStatus::Forwarded => write!(f, "forwarded"),
            MailMessageStatus::ForwardFailed => write!(f, "forward_failed"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MailApp {
    pub id: i32,
    pub name: String,
    pub domain: String,
    pub aws_region: String,
    pub project_id: i32,
    pub tenant_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: MailStatus,
    pub ses_inbound_mx_value: String,
    pub inbound_bucket_name: String,
    pub inbound_email_key_prefix: String,
    #[serde(default)]
    pub lambda_config: HashMap<String, String>,
    #[serde(default)]
    pub inboxes: Vec<MailInbox>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MailInbox {
    pub id: i32,
    pub local_part: String,
    pub full_address: String,
    pub inbox_email: String,
    pub sender_email: String,
    pub forward_to_email: String,
    pub mail_app_id: i32,
    pub project_id: i32,
    pub tenant_id: i32,
    pub last_test_email_sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: MailStatus,
    pub email_key_prefix: String,
    #[serde(default)]
    pub lambda_config: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MailMessage {
    pub id: i32,
    pub mail_inbox_id: i32,
    pub provider_message_id: String,
    pub original_recipient_email: String,
    pub from_email: String,
    pub from_name: Option<String>,
    pub subject: Option<String>,
    pub text_preview: Option<String>,
    pub size_bytes: Option<i64>,
    pub forwarded_at: Option<DateTime<Utc>>,
    pub forward_error: Option<String>,
    pub received_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: MailMessageStatus,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    #[serde(default)]
    pub to_emails: Vec<String>,
    #[serde(default)]
    pub cc_emails: Vec<String>,
    #[serde(default)]
    pub forward_recipients: Vec<String>,
    #[serde(default)]
    pub headers_json: HashMap<String, String>,
    pub s3_bucket: Option<String>,
    pub s3_key: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct MailAppCreate {
    pub name: String,
    pub project_id: String,
    pub domain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_region: Option<String>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct MailAppUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_region: Option<String>,
}

impl MailAppUpdate {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.domain.is_none() && self.aws_region.is_none()
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct MailInboxCreate {
    pub local_part: String,
    pub forward_to_email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_email: Option<String>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct MailInboxUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_part: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_to_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_email: Option<String>,
}

impl MailInboxUpdate {
    pub fn is_empty(&self) -> bool {
        self.local_part.is_none() && self.forward_to_email.is_none() && self.sender_email.is_none()
    }
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct MailTestEmailRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MailTestEmailDelivery {
    pub message_id: String,
    pub recipient_email: String,
    pub delivery_method: String,
    pub sent_at: String,
}
