# smbCloud SDK Auth

Python auth SDK for smbCloud.

## About

`smbcloud-sdk-auth` wraps the shared Rust auth crate through PyO3 so Python apps can use the same auth API as the browser SDK.

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

For the wider platform and docs, see [smbCloud](https://smbcloud.xyz/) and the [developer guides](https://smbcloud.xyz/posts).

## License

Apache-2.0

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
