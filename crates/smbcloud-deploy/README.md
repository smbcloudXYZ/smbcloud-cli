# smbcloud-deploy

The build and deploy engine behind [smbCloud](https://smbcloud.xyz) and the
[`smb` CLI](https://github.com/smbcloudXYZ/smbcloud-cli).

`smbcloud-deploy` takes one app from a repo and ships it: detect the runtime,
build it, and transfer the result to the server. The engine is UI-agnostic and
auth-agnostic, so the same code drives the interactive CLI, CI pipelines, and the
server-side git receiver. They differ only in how they report progress and how
they authenticate.

## Design

Two things are inverted so the engine stays reusable:

- `Reporter` takes the place of direct terminal output. The engine reports steps;
  the caller decides how to render them. The CLI uses spinners, CI prints plain
  lines, and the server streams output back over the git push.
- Authentication is passed in. The engine never reads local credentials or
  prompts for login; a token or credentials come from the caller.

Transport sits behind a `Transport` trait, so rsync today and git-smart-HTTP
later are swappable without changing any caller.

## Status

Early and moving. This crate is extracted from the `smb` CLI so the deploy logic
lives in one place. The API will change while the surface settles.

## Part of smbCloud

smbCloud is a local-first build and deployment platform for shipping web apps,
APIs, and inference services from a repo to production. You build locally, push
the artifact, and smbCloud places it and serves it.

- Homepage: [smbcloud.xyz](https://smbcloud.xyz)
- CLI and source: [github.com/smbcloudXYZ/smbcloud-cli](https://github.com/smbcloudXYZ/smbcloud-cli)

## License

[Apache-2.0](LICENSE)

## Copyright

© 2026 [Splitfire AB](https://5mb.app) ([smbCloud](https://smbcloud.xyz)).
