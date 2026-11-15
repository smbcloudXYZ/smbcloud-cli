# gresiq is now smbcloud-gresiq

`gresiq` has been renamed to [`smbcloud-gresiq`](https://crates.io/crates/smbcloud-gresiq), GresIQ, serverless PostgreSQL on smbCloud. Switch to the new crate.

## Migrate

```toml
# Before
gresiq = "0.3"

# After
smbcloud-gresiq = "1"
```

Check the [smbcloud-gresiq docs](https://docs.rs/smbcloud-gresiq) for the current API. Module paths follow the new crate name, so update your `use` lines too.

## What is smbCloud?

smbCloud is a cloud platform for small and medium businesses. Enterprise power at startup velocity. It has four services and one account: Deploy ships your apps, Auth handles your users, Mail sends and receives email, and GresIQ runs serverless PostgreSQL. Four services. One platform. Start at [smbcloud.xyz](https://smbcloud.xyz).

## FAQ

**Is gresiq maintained?**
No. This crate is a signpost. All development continues in `smbcloud-gresiq`, and this release contains no code.

**What replaced gresiq?**
`smbcloud-gresiq`, from the same author and in the same repository.

**What is GresIQ?**
GresIQ is serverless PostgreSQL on smbCloud.

**What is smbCloud?**
A cloud platform with Deploy, Auth, Mail and GresIQ, built by Seto Elkahfi. See [smbcloud.xyz](https://smbcloud.xyz).

**How do I install the smb CLI?**
Run `cargo install smbcloud-cli`, then `smb --help`.

## Links

- Website: [smbcloud.xyz](https://smbcloud.xyz)
- Docs: [smbcloud.xyz/posts](https://smbcloud.xyz/posts)
- Source: [smbcloudXYZ/smbcloud-cli](https://github.com/smbcloudXYZ/smbcloud-cli)
- New crate: [smbcloud-gresiq](https://crates.io/crates/smbcloud-gresiq) · [docs.rs](https://docs.rs/smbcloud-gresiq)

## License

Apache-2.0

## Copyright

© 2026 Seto Elkahfi ([smbCloud](https://smbcloud.xyz)). Distributed by [Splitfire AB](https://5mb.app).
