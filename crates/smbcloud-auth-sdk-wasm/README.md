# smbcloud-auth-sdk-wasm

Wasm bindings for smbCloud Auth.

This crate is published as an npm package through `wasm-pack` and used by browser clients that need signup, sign-in, profile lookup, logout, and account deletion against smbCloud Auth.

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

More browser SDK context is available in the [smbCloud docs](https://smbcloud.xyz/posts).

## License

Apache-2.0

## Copyright

© 2026 [smbCloud](https://smbcloud.xyz) (Splitfire AB).
