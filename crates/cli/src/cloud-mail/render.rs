use console::style;
use smbcloud_model::mail::{MailApp, MailInbox, MailMessage, MailTestEmailDelivery};
use std::collections::HashMap;

fn print_heading(title: &str) {
    println!("\n{}", style(title).bold().underlined());
}

fn print_field(label: &str, value: impl AsRef<str>) {
    println!("{} {}", style(format!("{label}:")).cyan(), value.as_ref());
}

fn print_optional_field(label: &str, value: Option<&str>) {
    if let Some(value) = value {
        print_field(label, value);
    }
}

fn print_string_map(title: &str, values: &HashMap<String, String>) {
    if values.is_empty() {
        return;
    }

    print_heading(title);
    let mut pairs: Vec<_> = values.iter().collect();
    pairs.sort_by(|left, right| left.0.cmp(right.0));

    for (key, value) in pairs {
        print_field(key, value);
    }
}

pub(crate) fn print_mail_apps(mail_apps: &[MailApp]) {
    print_heading("Mail apps");

    if mail_apps.is_empty() {
        println!("No mail apps found.");
        return;
    }

    for mail_app in mail_apps {
        println!(
            "#{} {} ({}) [{}] project={} inboxes={}",
            mail_app.id,
            style(&mail_app.name).bold(),
            mail_app.domain,
            mail_app.status,
            mail_app.project_id,
            mail_app.inboxes.len()
        );
    }
}

pub(crate) fn print_mail_app_detail(mail_app: &MailApp) {
    print_heading("Mail app");
    print_field("ID", mail_app.id.to_string());
    print_field("Name", &mail_app.name);
    print_field("Domain", &mail_app.domain);
    print_field("Status", mail_app.status.to_string());
    print_field("AWS region", &mail_app.aws_region);
    print_field("Project ID", mail_app.project_id.to_string());
    print_field("Tenant ID", mail_app.tenant_id.to_string());
    print_field("SES inbound MX", &mail_app.ses_inbound_mx_value);
    print_field("Inbound bucket", &mail_app.inbound_bucket_name);
    print_field("Inbound prefix", &mail_app.inbound_email_key_prefix);
    print_field("Created at", mail_app.created_at.to_rfc3339());
    print_field("Updated at", mail_app.updated_at.to_rfc3339());
    print_string_map("Lambda config", &mail_app.lambda_config);

    print_heading("Inboxes");
    if mail_app.inboxes.is_empty() {
        println!("No inboxes configured.");
        return;
    }

    for inbox in &mail_app.inboxes {
        println!(
            "#{} {} -> {} [{}]",
            inbox.id,
            style(&inbox.full_address).bold(),
            inbox.forward_to_email,
            inbox.status
        );
    }
}

pub(crate) fn print_mail_inbox_detail(mail_inbox: &MailInbox) {
    print_heading("Mail inbox");
    print_field("ID", mail_inbox.id.to_string());
    print_field("Mail app ID", mail_inbox.mail_app_id.to_string());
    print_field("Project ID", mail_inbox.project_id.to_string());
    print_field("Tenant ID", mail_inbox.tenant_id.to_string());
    print_field("Local part", &mail_inbox.local_part);
    print_field("Inbox email", &mail_inbox.inbox_email);
    print_field("Forward target", &mail_inbox.forward_to_email);
    print_field("Sender email", &mail_inbox.sender_email);
    print_field("Status", mail_inbox.status.to_string());
    print_field("Email key prefix", &mail_inbox.email_key_prefix);
    print_field("Created at", mail_inbox.created_at.to_rfc3339());
    print_field("Updated at", mail_inbox.updated_at.to_rfc3339());
    print_optional_field(
        "Last test email sent at",
        mail_inbox
            .last_test_email_sent_at
            .as_ref()
            .map(|value| value.to_rfc3339())
            .as_deref(),
    );
    print_string_map("Lambda config", &mail_inbox.lambda_config);
}

pub(crate) fn print_mail_test_delivery(delivery: &MailTestEmailDelivery) {
    print_heading("Mail test delivery");
    print_field("Message ID", &delivery.message_id);
    print_field("Recipient", &delivery.recipient_email);
    print_field("Delivery method", &delivery.delivery_method);
    print_field("Sent at", &delivery.sent_at);
}

pub(crate) fn print_mail_messages(messages: &[MailMessage]) {
    print_heading("Mail messages");

    if messages.is_empty() {
        println!("No mail messages found.");
        return;
    }

    for message in messages {
        println!(
            "#{} {} from {} [{}]",
            message.id,
            message
                .subject
                .clone()
                .unwrap_or_else(|| "(no subject)".to_string()),
            message.from_email,
            message.status
        );
        if let Some(text_preview) = &message.text_preview {
            println!("  {}", text_preview);
        }
    }
}

pub(crate) fn print_mail_message_detail(message: &MailMessage) {
    print_heading("Mail message");
    print_field("ID", message.id.to_string());
    print_field("Inbox ID", message.mail_inbox_id.to_string());
    print_field("Status", message.status.to_string());
    print_field("Provider message ID", &message.provider_message_id);
    print_field("Original recipient", &message.original_recipient_email);
    print_field("From email", &message.from_email);
    print_optional_field("From name", message.from_name.as_deref());
    print_optional_field("Subject", message.subject.as_deref());
    print_optional_field("Text preview", message.text_preview.as_deref());
    print_optional_field("Forward error", message.forward_error.as_deref());
    print_field("Received at", message.received_at.to_rfc3339());
    print_optional_field(
        "Forwarded at",
        message
            .forwarded_at
            .as_ref()
            .map(|value| value.to_rfc3339())
            .as_deref(),
    );
    print_optional_field(
        "Size bytes",
        message.size_bytes.map(|value| value.to_string()).as_deref(),
    );
    if !message.to_emails.is_empty() {
        print_field("To", message.to_emails.join(", "));
    }
    if !message.cc_emails.is_empty() {
        print_field("CC", message.cc_emails.join(", "));
    }
    if !message.forward_recipients.is_empty() {
        print_field("Forward recipients", message.forward_recipients.join(", "));
    }
    print_optional_field("S3 bucket", message.s3_bucket.as_deref());
    print_optional_field("S3 key", message.s3_key.as_deref());
    print_optional_field("Text body", message.text_body.as_deref());
    print_optional_field("HTML body", message.html_body.as_deref());
    print_string_map("Headers", &message.headers_json);
}
