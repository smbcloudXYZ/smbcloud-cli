---
name: smbcloud-ruby-native-gem
description: Use when building, packaging, or publishing smbCloud Ruby gems backed by Rust native extensions through Magnus and rb_sys, especially under sdk/gems/model and sdk/gems/auth. Covers gem layout, path wiring to workspace crates, rake build/compile flows, pkg output, .gitignore conventions, and RubyGems release preflight.
---

# smbCloud Ruby Native Gem

Use this skill when work touches smbCloud Ruby gems that wrap Rust crates through Magnus.

Applies to:

- `sdk/gems/model`
- `sdk/gems/auth`
- new gems following the same pattern
- `ext/*/Cargo.toml` path wiring
- `auth.gemspec` or `model.gemspec`
- `Rakefile`, `extconf.rb`, `lib/*.rb`, `sig/*.rbs`
- local build and RubyGems publish steps

## Source of truth

For each gem, inspect these files first:

- `sdk/gems/<name>/<name>.gemspec`
- `sdk/gems/<name>/Rakefile`
- `sdk/gems/<name>/ext/<name>/Cargo.toml`
- `sdk/gems/<name>/ext/<name>/extconf.rb`
- `sdk/gems/<name>/lib/<name>.rb`
- `sdk/gems/<name>/lib/<name>/version.rb`
- `sdk/gems/<name>/.gitignore`

The Rust crate under `crates/` remains the API source of truth. The gem should stay a thin Ruby-facing adapter.

## Expected layout

Use the same structure as `sdk/gems/model` and `sdk/gems/auth`:

- workspace `Cargo.toml` at gem root
- native extension crate at `ext/<name>`
- Ruby entrypoint at `lib/<name>.rb`
- version file at `lib/<name>/version.rb`
- type signature file under `sig/`
- `Rakefile` using `RbSys::ExtensionTask`
- `extconf.rb` using `create_rust_makefile`

Do not invent a different packaging layout unless there is a strong reason.

## Cargo path wiring

Be careful with relative paths from `sdk/gems/<name>/ext/<name>/Cargo.toml` to workspace crates.

From that directory, `crates/...` is typically five levels up:

- `./../../../../../crates/<crate-name>`

Do not assume four levels. Verify the actual path before building.

## Ruby API design

Keep the gem framework-agnostic.

Preferred layering:

1. Rust crate owns transport and domain logic
2. native Magnus extension bridges into Ruby
3. plain Ruby wrapper exposes the public API

For reusable SDKs, prefer a client object such as:

- `SmbCloud::Auth::Client.new(...)`

Keep Rails-specific helpers, sessions, and controller concerns out of the core gem. If needed later, create a separate Rails adapter gem.

## Build workflow

Use the standard rake tasks.

From `sdk/gems/<name>`:

- `bundle install`
- `bundle exec rake compile`
- `bundle exec rake build`

Expected behavior:

- native build output goes under `target/`
- packaged gem goes under `pkg/`

For Rust-only verification, also use:

- `cargo check`
- `cargo fmt --manifest-path ext/<name>/Cargo.toml`

Prefer formatting the extension crate directly if the parent workspace has unrelated manifest issues.

## .gitignore conventions

Match the `sdk/gems/model` pattern.

Ignore:

- `/.bundle/`
- `/pkg/`
- `/tmp/`
- `target/`
- `*.bundle`
- native object outputs such as `*.so`, `*.o`, `*.a`
- `mkmf.log`

Do not track built `.gem` artifacts in git. They belong in `pkg/` and should be ignored.

## Release preflight

Before publishing:

1. confirm the version in `lib/<name>/version.rb`
2. build the gem with `bundle exec rake build`
3. verify the artifact exists in `pkg/`
4. inspect warnings from `gem build` or `rake build`
5. confirm RubyGems credentials exist

Credentials can come from either:

- `~/.gem/credentials`
- `GEM_HOST_API_KEY`

Without one of those, `gem push` will fail.

## Publish

From `sdk/gems/<name>`:

- `bundle exec gem push pkg/<name>-<version>.gem`

Do not claim a publish succeeded unless the push completed successfully.

## Common mistakes

- wrong relative path from `ext/<name>/Cargo.toml` to `crates/...`
- exposing Ruby methods that the native extension does not actually export
- putting release artifacts in git
- making the gem Rails-specific when the crate should stay generic
- using `gem build` ad hoc while ignoring the repo's `rake build` and `pkg/` convention
- attempting `gem push` without RubyGems credentials configured
