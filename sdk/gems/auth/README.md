# smbcloud-auth Gem

Ruby bindings for the smbCloud Auth SDK.

## Install

```bash
gem install smbcloud-auth
```

## Usage

```ruby
require "auth"

client = SmbCloud::Auth::Client.new(
  environment: SmbCloud::Auth::Environment::PRODUCTION,
  app_id: "app-id",
  app_secret: "app-secret"
)

signup = client.signup(
  email: "name@example.com",
  password: "password123"
)

login = client.login(
  email: "name@example.com",
  password: "password123"
)

me = client.me(access_token: login[:access_token])
```

## API

`SmbCloud::Auth::Client` exposes:

- `signup(email:, password:)`
- `login(email:, password:)`
- `me(access_token:)`
- `logout(access_token:)`
- `remove(access_token:)`

For simple scripting, module-level convenience wrappers are also available:

- `SmbCloud::Auth.signup_with_client(...)`
- `SmbCloud::Auth.login_with_client(...)`
- `SmbCloud::Auth.me_with_client(...)`
- `SmbCloud::Auth.logout_with_client(...)`
- `SmbCloud::Auth.remove_with_client(...)`

> Explore more on the [smbCloud Services](https://smbcloud.xyz/services) page.

## License

MIT

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
