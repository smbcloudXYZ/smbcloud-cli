# smbcloud-auth-sdk-wasm

Wasm bindings for the smbCloud Auth SDK.

This crate is intended to be published as an npm package via `wasm-pack` and
consumed by browser clients that need tenant auth-app signup, sign-in, profile
lookup, and account deletion against the smbCloud Auth service.

## Exports

- `signup_with_client`
- `login_with_client`
- `logout_with_client`
- `me_with_client`
- `remove_with_client`
- `Environment`

## Build

```bash
wasm-pack build --target web --release
```

## Browser usage

```js
import init, {
  Environment,
  login_with_client,
  signup_with_client,
} from "smbcloud-auth-sdk-wasm";

await init();

await signup_with_client(
  Environment.Production,
  "app-id",
  "app-secret",
  "name@example.com",
  "password123",
);
```
