# ADR

This repo uses [Architecture Decision Record](https://github.com/joelparkerhenderson/architecture-decision-record) in the `adr` folder.

# Publish Update

There's no GitHub action to automatically publish the package for now. Publishing task can also be done locally.

To update the crate in the [crate.io](https://crate.io), use [cargo workspaces](https://github.com/pksunkara/cargo-workspaces) subcommand.

Steps to publish new package:

```bash
$ cargo workspaces publish --publish-as-is
```

It will ask whether should bump the vesions of the packages or not. To update version separately:

```bash
$ cargo workspaces version
```
