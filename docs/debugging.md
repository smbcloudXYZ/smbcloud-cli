# VSCode Debugging

To debug the app on VSCode, use [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb) and [cargo debugger](https://github.com/jkelleyrtp/cargo-debugger).

```bash
$ cargo debugger --manifest-path smbcloud-cli/Cargo.toml -- account signup
```


## Environment

Default environment is production. To run against dev environemt:
```bash
$ cargo run -- -e dev account login
```