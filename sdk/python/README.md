# smbCloud SDK Auth

Python authentication SDK for smbCloud, built from Rust.

## About

`smbcloud-sdk-auth` wraps the shared Rust Auth crate through PyO3 and maturin so
Python apps can use the same tenant auth contract as the browser SDK.

Current exported Python APIs include:

- `signup_with_client`
- `login_with_client`
- `logout_with_client`
- `me_with_client`
- `remove_with_client`
- `Environment`
- `AuthClient`

## Install

```bash
pip install smbcloud-sdk-auth
```

## Usage

```python
from smbcloud_auth import AuthClient, Environment

client = AuthClient(
    env=Environment.PRODUCTION,
    app_id="app-id",
    app_secret="app-secret",
)

signup = client.signup("name@example.com", "password123")
login = client.login("name@example.com", "password123")
user = client.me(login["access_token"])
```

## Local packaging

From `sdk/python`:

```bash
maturin build
```

> Explore more on the [smbCloud Services](https://smbcloud.xyz/services) page.

## License

Apache-2.0

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
