# Publishing the `smbcloud-model` gem

The gem wraps the `smbcloud-model` Rust crate via a native extension. Publishing requires coordinating the Rust crate release and the gem release in order.

## Prerequisites

- [Rust toolchain](https://rustup.rs) installed
- `bundler` and `gem` CLI available
- RubyGems account with push access to `smbcloud-model`

---

## Steps

### 1. Publish the Rust crate

Publish `smbcloud-model` to crates.io first. The gem's native extension depends on it.

```sh
cargo publish -p smbcloud-model
```

### 2. Update the Cargo dependency

Switch `gems/model/ext/model/Cargo.toml` from the local path to the published version:

```toml
# Before (development)
smbcloud-model = { path = "./../../../../crates/smbcloud-model" }

# After (release) — match the version just published
smbcloud-model = "0.3"
```

### 3. Sync the gem version

Set `gems/model/lib/model/version.rb` to match the `smbcloud-model` crate version:

```ruby
module Model
  VERSION = '0.3.31'
end
```

### 4. Compile the native extension

```sh
cd gems/model
bundle exec rake compile
```

### 5. Build the gem

```sh
bundle exec rake build
```

This produces `pkg/smbcloud-model-<version>.gem`.

### 6. Publish the gem

```sh
gem push pkg/smbcloud-model-<version>.gem
```

---

## Development workflow

To work against the local crate instead of the published one, use the path dependency:

```toml
smbcloud-model = { path = "./../../../../crates/smbcloud-model" }
```

Remember to revert this before publishing.
