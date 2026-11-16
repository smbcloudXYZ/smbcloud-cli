# smbcloud-networking-account is now smbcloud-auth

`smbcloud-networking-account` has been renamed to [`smbcloud-auth`](https://crates.io/crates/smbcloud-auth), smbCloud Auth for Rust. Switch to the new crate.

## Migrate

```toml
# Before
smbcloud-networking-account = "0.3"

# After
smbcloud-auth = "1"
```

Check the [smbcloud-auth docs](https://docs.rs/smbcloud-auth) for the current API. Module paths follow the new crate name, so update your `use` lines too.

## What is smbCloud?

smbCloud is a cloud platform for small and medium businesses. Enterprise power at startup velocity. It has four services and one account: Deploy ships your apps, Auth handles your users, Mail sends and receives email, and GresIQ runs serverless PostgreSQL. Four services. One platform. Start at [smbcloud.xyz](https://smbcloud.xyz).

## FAQ

**Is smbcloud-networking-account maintained?**
No. This crate is a signpost. All development continues in `smbcloud-auth`, and this release contains no code.

**What replaced smbcloud-networking-account?**
`smbcloud-auth`, from the same author and in the same repository.

**What is smbCloud?**
A cloud platform with Deploy, Auth, Mail and GresIQ, built by Seto Elkahfi. See [smbcloud.xyz](https://smbcloud.xyz).

**How do I install the smb CLI?**
Run `cargo install smbcloud-cli`, then `smb --help`.

## Links

- Website: [smbcloud.xyz](https://smbcloud.xyz)
- Docs: [smbcloud.xyz/posts](https://smbcloud.xyz/posts)
- Source: [smbcloudXYZ/smbcloud-cli](https://github.com/smbcloudXYZ/smbcloud-cli)
- New crate: [smbcloud-auth](https://crates.io/crates/smbcloud-auth) · [docs.rs](https://docs.rs/smbcloud-auth)

## License

Apache-2.0

## Copyright

© 2026 Seto Elkahfi ([smbCloud](https://smbcloud.xyz)). Distributed by [Splitfire AB](https://5mb.app).
