# smbpndk-networking-project is now smbcloud-networking-project

`smbpndk-networking-project` has been renamed to [`smbcloud-networking-project`](https://crates.io/crates/smbcloud-networking-project), the smbCloud project and deployment API client. Switch to the new crate.

## Migrate

```toml
# Before
smbpndk-networking-project = "0.3"

# After
smbcloud-networking-project = "1"
```

Check the [smbcloud-networking-project docs](https://docs.rs/smbcloud-networking-project) for the current API. Module paths follow the new crate name, so update your `use` lines too.

## What is smbCloud?

smbCloud is a cloud platform for small and medium businesses. Enterprise power at startup velocity. It has four services and one account: Deploy ships your apps, Auth handles your users, Mail sends and receives email, and GresIQ runs serverless PostgreSQL. Four services. One platform. Start at [smbcloud.xyz](https://smbcloud.xyz).

## FAQ

**Is smbpndk-networking-project maintained?**
No. This crate is a signpost. All development continues in `smbcloud-networking-project`, and this release contains no code.

**What replaced smbpndk-networking-project?**
`smbcloud-networking-project`, from the same author and in the same repository.

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
- New crate: [smbcloud-networking-project](https://crates.io/crates/smbcloud-networking-project) · [docs.rs](https://docs.rs/smbcloud-networking-project)

## License

Apache-2.0

## Copyright

© 2026 Seto Elkahfi ([smbCloud](https://smbcloud.xyz)). Distributed by [Splitfire AB](https://5mb.app).
