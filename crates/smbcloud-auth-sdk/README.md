# smbcloud-auth-sdk

Client SDK for [smbCloud Auth](https://smbcloud.xyz/) — lets you authenticate your users against the smbCloud Auth service using your app credentials (`client_id` / `client_secret`).

## Usage

```rust
use smbcloud_auth_sdk::{
    client_credentials::ClientCredentials,
    login::login_with_client,
    signup::signup_with_client,
    me::me_with_client,
};
use smbcloud_network::environment::Environment;

let client = ClientCredentials {
    app_id: "your-app-id",
    app_secret: "your-app-secret",
};

// Sign up
let result = signup_with_client(Environment::Production, client, email, password).await?;

// Sign in
let status = login_with_client(Environment::Production, client, email, password).await?;

// Profile
let user = me_with_client(Environment::Production, client, &access_token).await?;
```

> Explore more on the [smbCloud Services](https://smbcloud.xyz/services) page.

## License

Apache-2.0

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
