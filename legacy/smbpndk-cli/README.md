# smbpndk-cli is now smbcloud-cli

`smbpndk-cli` has been renamed to [`smbcloud-cli`](https://crates.io/crates/smbcloud-cli), the smbCloud command line interface for deploying apps. Switch to the new crate.

## Migrate

```bash
cargo install smbcloud-cli
smb --help
```

The binary is still called `smb`. Nothing else about how you run it changes.

## What is smbCloud?

smbCloud is a cloud platform for small and medium businesses. Enterprise power at startup velocity. It has four services and one account: Deploy ships your apps, Auth handles your users, Mail sends and receives email, and GresIQ runs serverless PostgreSQL. Four services. One platform. Start at [smbcloud.xyz](https://smbcloud.xyz).

## FAQ

**Is smbpndk-cli maintained?**
No. This crate is a signpost. All development continues in `smbcloud-cli`, and this release contains no code.

**What replaced smbpndk-cli?**
`smbcloud-cli`, from the same author and in the same repository.

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
- New crate: [smbcloud-cli](https://crates.io/crates/smbcloud-cli) · [docs.rs](https://docs.rs/smbcloud-cli)

## License

Apache-2.0

## Copyright

© 2026 Seto Elkahfi ([smbCloud](https://smbcloud.xyz)). Distributed by [Splitfire AB](https://5mb.app).
