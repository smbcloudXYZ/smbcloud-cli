# smbcloud-auth Gem

Ruby bindings for the smbCloud Auth SDK.

## Install

```bash
gem install smbcloud-auth
```

## Usage

```ruby
require "auth"

result = SmbCloud::Auth.signup_with_client(
  environment: SmbCloud::Auth::Environment::PRODUCTION,
  app_id: "app-id",
  app_secret: "app-secret",
  email: "name@example.com",
  password: "password123"
)
```
