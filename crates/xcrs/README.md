# XCRS Mobile & TV Automation

Cross-platform MCP tools for testing mobile and TV apps on Apple simulators,
physical Apple devices, Android phones, and Android TV devices. XCRS combines
Xcode, ControlKit, CoreDevice, and adb behind one target-aware automation
workflow.

MCP Registry name: `mcp-name: io.github.smbcloudXYZ/xcrs`

## Run the MCP server

```sh
cargo install xcrs
xcrs --mcp
```

Android tools use `adb` from `ANDROID_SDK_ROOT`, `ANDROID_HOME`, the standard
Android SDK locations, or `PATH`. Connect a device with USB debugging enabled
and authorize the host before selecting it with `device_select`.

For Android accessibility-tree inspection and semantic `ui_tap`, install the
optional [XCRS AndroidKit](../../docs/androidkit.md) companion runner. Raw adb
actions remain available without it.

The same automation profile is available from the full smbCloud CLI:

```sh
smb --mcp --scope automation
```

## Repository

Source and documentation are available in the
[smbcloud-cli repository](https://github.com/smbcloudXYZ/smbcloud-cli/tree/main/crates/xcrs).
