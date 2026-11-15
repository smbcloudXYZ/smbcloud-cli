# Legacy crate stubs

These folders hold the final release of crates that were renamed or retired. Each one is a docs-only crate (no code, no dependencies) at version `0.4.0`.

crates.io only refreshes a crate's page when a new version is published. Publishing one last release lets every old page say where the project went instead of showing stale text. Nothing is yanked, because yanking hides the page we want people to land on.

Version `0.4.0` is deliberate: anyone pinned to `^0.3` never picks it up.

| Legacy crate | Now |
| --- | --- |
| smbpndk-cli | smbcloud-cli |
| smbpndk-model | smbcloud-model |
| smbpndk-networking | smbcloud-networking |
| smbpndk-networking-project | smbcloud-networking-project |
| smbpndk-utils | smbcloud-utils |
| app-oten | smbcloud-auth |
| app-fun | reserved for smbCloud Functions |
| app-pkt | reserved for smbCloud Packages |
| smbcloud-networking-account | smbcloud-auth |
| smbcloud-networking-account-wasm | smbcloud-auth-sdk-wasm |
| smbcloud-auth-wasm | smbcloud-auth-sdk-wasm |
| smbcloud-auth-py | smbcloud-auth-sdk-py |
| gresiq | smbcloud-gresiq |
| s6n | smbcloud-s6n |

These crates are excluded from the workspace (`exclude = ["legacy"]` in the root `Cargo.toml`) and never build with the CLI. To publish one, run `cargo publish --dry-run` and then `cargo publish` from its folder.
