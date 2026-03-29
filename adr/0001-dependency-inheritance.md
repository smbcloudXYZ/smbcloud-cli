# ADR 0001: Dependency Inheritance

- Status: Accepted
- Date: 2025-04-19

## Context

The smbcloud-cli workspace contains multiple crates that share a large set of common dependencies.
Without a workspace-level dependency strategy, individual `Cargo.toml` files repeat the same
version declarations and features, which increases maintenance cost and creates version drift.

## Decision

We use Cargo workspace dependency inheritance for shared dependencies.

The workspace defines common dependencies in `[workspace.dependencies]`, and member crates inherit
those dependencies instead of repeating version declarations locally.

We use `cargo autoinherit` to apply and maintain this inheritance pattern.

Reference:
- https://mainmatter.com/blog/2024/03/18/cargo-autoinherit

## Consequences

Positive:
- Shared dependency versions are managed in one place.
- Workspace manifests are easier to keep consistent.
- Upgrades are simpler because most dependency changes happen at the workspace root.

Tradeoffs:
- Contributors need to understand workspace inheritance when editing crate manifests.
- Tooling and review need to account for inherited dependencies instead of crate-local versions.
