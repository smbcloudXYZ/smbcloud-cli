# smbcloud-email (Ruby)

Ruby bindings for the **smbCloud transactional email API**, powered by the shared
Rust SDK ([`smbcloud-email-sdk`](../../../crates/smbcloud-email-sdk)) and a native
[Magnus](https://github.com/matsadler/magnus) extension.

## Install

```ruby
gem 'smbcloud-email'
```

The gem ships a native extension; `bundle install` compiles it (Rust toolchain
required).

## Usage

```ruby
require 'email'

client = SmbCloud::Email.client(
  environment: SmbCloud::Email::Environment::PRODUCTION,
  api_key: 'smb_mail_your_key',
)

# Send
sent = client.send(
  from: 'billing@example.com',
  to: ['customer@acme.com'],
  subject: 'Your receipt',
  html: '<h1>Thanks!</h1>',
  text: 'Thanks!',
  idempotency_key: 'receipt-2026-0001',
)
sent[:id]      # => "eml_…"
sent[:status]  # => 1 (sent)

# Attachments, cc/bcc, headers, tags
client.send(
  from: 'billing@example.com',
  to: ['customer@acme.com'],
  subject: 'Invoice',
  html: '<p>See attached.</p>',
  cc: ['accounts@acme.com'],
  attachments: [{ filename: 'invoice.pdf', content_base64: 'JVBERi0xLjQ...' }],
  headers: { 'X-Entity-Ref-ID' => 'inv_123' },
  tags: { 'category' => 'invoice' },
)

# Delivery status (needs a read-scope key)
message = client.get_message('eml_…')
message[:status]
message[:events]

client.list_messages(status: 'bounced', limit: 20)
```

Mint an API key for your Mail app in the smbCloud console. Sending is scoped to
that app's verified domain; reading messages needs a read-scope key.

## Errors

Non-2xx API responses and transport failures raise `SmbCloud::Email::Error`, with
the parsed response body available on `#payload`.

## License

MIT
