# Changelog

## [0.4.7] - 2026-06-30

- Initial release. Ruby bindings for the smbCloud transactional email API:
  `SmbCloud::Email::Client#send`, `#get_message`, and `#list_messages`,
  powered by the shared Rust SDK (`smbcloud-email-sdk`) via a native Magnus
  extension.
