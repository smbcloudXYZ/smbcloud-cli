# Publish gem

To publish the gem with native extension, there are multiple steps to do.

## Build the gem

- Make sure to publish the cli first since it will publish the `smbcloud-model` crate as well, which the gem depends on.
- Update the Cargo dependency in `gems/model/ext/model/Cargo.toml`.
- Build the gem: `bundle exec rake compile`.
- Update the gem version in the `gem/model/lib/model/version.rb`. Just use the same version as the `smbcloud-model` crate.
- Compile or build the gem directly: `bundle exec rake compile` or ` bundle exec rake buid`.
- Then publish the gem: `gem push pkg/smbcloud-model-{version}.gem`.