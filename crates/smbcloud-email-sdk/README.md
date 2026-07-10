# smbcloud-email-sdk

Rust client for the **smbCloud transactional email API**. Send transactional
email from a verified domain and read each message's delivery status.

The crate handles the HTTP transport and the `Authorization: Bearer <api_key>`
header. You build the message and own the content.

## Install

```toml
[dependencies]
smbcloud-email-sdk = "0.4"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Authentication

Mint a `smb_mail_…` API key for your Mail app in the smbCloud console. Sending is
scoped to that app's verified domain; reading messages needs a read-scope key.

## Send

```rust,no_run
use smbcloud_email_sdk::{EmailClient, EmailCredentials, Environment, SendEmail};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = EmailClient::from_credentials(
        Environment::Production,
        EmailCredentials { api_key: "smb_mail_your_key" },
    );

    let message = SendEmail::new("billing@example.com", ["customer@acme.com"])
        .subject("Your receipt")
        .html("<h1>Thanks!</h1>")
        .text("Thanks!")
        .idempotency_key("receipt-2026-0001");

    let sent = client.send(&message).await?;
    println!("sent {} ({:?})", sent.id, sent.status);
    Ok(())
}
```

## Attachments, cc/bcc, headers, tags

```rust,no_run
# use smbcloud_email_sdk::{Attachment, SendEmail};
let message = SendEmail::new("billing@example.com", ["customer@acme.com"])
    .subject("Invoice")
    .html("<p>See attached.</p>")
    .cc(["accounts@acme.com"])
    .reply_to(["support@example.com"])
    .attachment(Attachment {
        filename: "invoice.pdf".into(),
        content_base64: "JVBERi0xLjQ...".into(),
    })
    .header("X-Entity-Ref-ID", "inv_123")
    .tag("category", "invoice");
```

## Read delivery status

```rust,no_run
# async fn run(client: smbcloud_email_sdk::EmailClient) -> anyhow::Result<()> {
let message = client.get_message("eml_…").await?;
println!("status: {:?}", message.status);
for event in &message.events {
    println!("  {} at {}", event.event_type, event.occurred_at);
}

let recent = client.list_messages(Some("bounced"), Some(20)).await?;
println!("{} bounced", recent.len());
# Ok(())
# }
```

## License

Apache-2.0
