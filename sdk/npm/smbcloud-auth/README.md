<!-- LOGO -->
<h1>
<p align="center">
  <img src="https://avatars.githubusercontent.com/u/89791739?s=200&v=4" alt="Logo" width="128">
  <br>smbCloud SDK Auth
</h1>
  <p align="center">
    Browser authentication SDK for smbCloud, built from Rust and WebAssembly.
    <br />
    <a href="README.md#about">About</a>
    ·
    <a href="README.md#install">Install</a>
    ·
    <a href="README.md#usage">Usage</a>
    ·
    <a href="README.md#license">License</a>
  </p>
</p>

## About

`@smbcloud/sdk-auth` is the browser SDK for smbCloud Auth. It wraps the shared Rust
account crate through WebAssembly so browser apps can use the same auth
contract as desktop and mobile clients.

Current exported browser APIs include:

- `signup_with_client`
- `login_with_client`
- `logout_with_client`
- `me_with_client`
- `remove_with_client`
- `Environment`

## Install

```bash
npm install @smbcloud/sdk-auth
```

## Usage

```js
import init, {
  Environment,
  login_with_client,
  signup_with_client,
} from "@smbcloud/sdk-auth";

await init();

await signup_with_client(
  Environment.Production,
  "app-id",
  "app-secret",
  "name@example.com",
  "password123",
);
```

## Local packaging

From `sdk/npm/smbcloud-auth`:

```bash
npm run prepare:package
npm run pack:dry-run
```

This builds the wasm crate, stages the generated artifacts into the npm package
folder, and lets you verify the publish payload before release.

## License

MIT

## Credits

2026 [smbCloud](https://github.com/smbCloudXYZ). Built for [SplitFire AI](https://splitfire.ai).
