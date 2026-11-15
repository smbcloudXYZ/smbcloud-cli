# smbpndk-utils is now smbcloud-utils

`smbpndk-utils` has been renamed to [`smbcloud-utils`](https://crates.io/crates/smbcloud-utils), shared helpers for smbCloud crates. Switch to the new crate.

## Migrate

```toml
# Before
smbpndk-utils = "0.3"

# After
smbcloud-utils = "1"
```

Check the [smbcloud-utils docs](https://docs.rs/smbcloud-utils) for the current API. Module paths follow the new crate name, so update your `use` lines too.

## What is smbCloud?

smbCloud is a cloud platform for small and medium businesses. Enterprise power at startup velocity. It has four services and one account: Deploy ships your apps, Auth handles your users, Mail sends and receives email, and GresIQ runs serverless PostgreSQL. Four services. One platform. Start at [smbcloud.xyz](https://smbcloud.xyz).

## FAQ

**Is smbpndk-utils maintained?**
No. This crate is a signpost. All development continues in `smbcloud-utils`, and this release contains no code.

**What replaced smbpndk-utils?**
`smbcloud-utils`, from the same author and in the same repository.

**What is smbpndk?**
smbpndk was the early name of smbCloud. The project, the CLI and the crates were renamed, and the `smbpndk-*` crates now point to their `smbcloud-*` equivalents.

**What is smbCloud?**
A cloud platform with Deploy, Auth, Mail and GresIQ, built by Seto Elkahfi. See [smbcloud.xyz](https://smbcloud.xyz).

**How do I install the smb CLI?**
Run `cargo install smbcloud-cli`, then `smb --help`.

## Links

- Website: [smbcloud.xyz](https://smbcloud.xyz)
- Docs: [smbcloud.xyz/posts](https://smbcloud.xyz/posts)
- Source: [smbcloudXYZ/smbcloud-cli](https://github.com/smbcloudXYZ/smbcloud-cli)
- New crate: [smbcloud-utils](https://crates.io/crates/smbcloud-utils) · [docs.rs](https://docs.rs/smbcloud-utils)

## License

Apache-2.0

## Copyright

© 2026 Seto Elkahfi ([smbCloud](https://smbcloud.xyz)). Distributed by [Splitfire AB](https://5mb.app).
