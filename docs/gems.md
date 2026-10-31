# Ruby gems

Three gems live under `sdk/gems/`, each a Magnus native extension over a Rust crate:

| Gem | Directory | Wraps |
| --- | --- | --- |
| `smbcloud-auth` | `sdk/gems/auth` | `smbcloud-auth-sdk` |
| `smbcloud-model` | `sdk/gems/model` | `smbcloud-model` |
| `smbcloud-email` | `sdk/gems/email` | `smbcloud-email-sdk` |

Gem versions move in lockstep with the CLI release, so `make patch | minor | major` bumps them (see [development.md](development.md)).

## Publishing

CI is the release path. [`release-sdk-gem.yml`](../.github/workflows/release-sdk-gem.yml) cross-compiles the native gems with `rb-sys-dock` (x86_64 and aarch64 Linux, arm64 and x86_64 darwin), builds a source gem, and pushes everything to RubyGems. [`release-crate.yml`](../.github/workflows/release-crate.yml) dispatches it after the crates are published.

There is no API key secret. The push job exchanges its GitHub OIDC token for a short-lived RubyGems key.

### Trusted publisher settings

Each gem needs a trusted publisher on rubygems.org (gem page, Trusted publishers), created by an owner of the gem:

| Field | Value |
| --- | --- |
| Owner | `smbcloudXYZ` |
| Repository | `smbcloud-cli` |
| Workflow filename | `release-sdk-gem.yml` |
| Environment | blank |

RubyGems matches the filename against the `job_workflow_ref` claim, which names the workflow that contains the running job. That is `release-sdk-gem.yml`, because `release-crate.yml` only dispatches it and a dispatch starts a separate workflow. A publisher registered to `release-crate.yml`, or to any other workflow, fails the "Configure RubyGems credentials" step with `No trusted publisher configured for this workflow`.

### First push of a new gem

A gem that is not on RubyGems yet cannot have a normal publisher. Create a pending one instead, from your profile under Pending trusted publishers, with the gem name and the same repository and workflow as above.

- A pending publisher expires after about 12 hours, so create it right before the release.
- The first push has to name the account that created it as `owner`, by its immutable ID. The workflow's credentials step passes this; the `rubygems/configure-rubygems-credentials` action has no input for it, which is why the step calls the exchange endpoint directly.
- The first successful push turns the pending publisher into a normal one.

### Completing a partial release

The push step skips any file RubyGems already has, so a failed run can be re-run without a new release or rebuild. Fix the cause (usually a publisher setting), then:

```sh
gh run rerun <run-id> --failed
```

Only the failed push jobs run again. The gem artifacts are kept for the Actions retention period. A re-run uses the workflow file from the original commit, so a change to the workflow itself needs a fresh `workflow_dispatch` instead.

Check the result with `https://rubygems.org/api/v1/versions/<gem>.json`: every platform gem and the source gem should list the version.

## Local build and development

Use `rv` for Ruby (`rv run bundle ...`). From a gem directory:

```sh
cd sdk/gems/model
rv run bundle install
rv run bundle exec rake compile
rv run bundle exec rake build   # writes pkg/smbcloud-model-<version>.gem
```

Each gem's extension crate (`ext/<gem>/Cargo.toml`) depends on the published `smbcloud-*` crate version. To build against your checkout instead, add a `[patch.crates-io]` section pointing at `crates/<name>` in a local, uncommitted Cargo config; CI does the same in its gem jobs.

A manual `gem push` is only for emergencies and needs local RubyGems credentials (`~/.gem/credentials` or `GEM_HOST_API_KEY`). Prefer CI.
