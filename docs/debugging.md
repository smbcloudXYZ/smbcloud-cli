# VSCode Debugging

To debug the app on VSCode, use [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) and [cargo debugger](https://github.com/jkelleyrtp/cargo-debugger).

```bash
$ cargo debugger --package smbcloud-cli -- account signup
```


## Environment

Default environment is production. To run against dev environemt:
```bash
$ cargo run -- -e dev account login
```

Example debug the dev environment:
```bash
$ cargo debugger --package cli -- -e dev account login
```

## Run from different directory

To run from a different directory, use the `--manifest-path` flag and `--package` flag. :
```bash
$ cargo debugger --manifest-path .../path/to/smbcloud-cli/Cargo.toml --package smbcloud-cli -- -e dev account login
```
