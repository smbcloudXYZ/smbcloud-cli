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

## Desktop apps

A desktop target drives an application through a W3C WebDriver endpoint, for example
`tauri-driver` for a Tauri app on Linux (it proxies WebKitWebDriver). Select it once:

```json
{ "name": "device_select", "arguments": { "webdriver_url": "http://127.0.0.1:4444" } }
```

`webdriver_url` cannot be combined with an Apple or Android field. The desktop tools then use the
stored target.

| Tool | Desktop behaviour |
|------|-------------------|
| `app_install_launch`, `app_launch` | Start a WebDriver session, which launches the binary at `app_path` (`app_id` for `app_launch`). `bundle_id` is ignored. |
| `app_terminate` | End the session and close the app. |
| `ui_describe` | The page source. |
| `ui_element_list` | Visible interactive elements with label, id, role and rectangle, up to 200. |
| `ui_tap` | Click the one displayed element whose id, name, test id, ARIA label, title or text equals `element`. Zero or several matches are an error. |
| `input_text` | Type into the focused element. |
| `screen_capture` | A PNG of the window. |

The other tools return an error saying they have no desktop meaning. A running app cannot be
attached to: the session is the launched app.

## Repository

Source and documentation are available in the
[smbcloud-cli repository](https://github.com/smbcloudXYZ/smbcloud-cli/tree/main/crates/xcrs).
