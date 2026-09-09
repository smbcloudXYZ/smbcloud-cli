# XCRS AndroidKit

XCRS AndroidKit is the optional Android companion runner for semantic E2E
automation. It is separate from adb: adb remains sufficient for screenshots,
coordinate input, app lifecycle, and system buttons.

Build an AndroidKit APK from the public `xcrs-androidkit` repository, then
install and start it on the selected Android device:

```sh
xcrs android-kit install --serial <serial> --apk <path-to-apk>
xcrs android-kit start --serial <serial>
xcrs android-kit status --serial <serial>
```

AndroidKit binds only to an Android local socket. xcrs creates and removes its
own `adb forward` connection for each RPC call; it does not expose a network
service on the device.

When AndroidKit is available, `ui_describe`, `ui_element_list`, and `ui_tap`
accept `app_id` as the Android package name. `ui_tap` uses an exact accessible
text, content description, resource identifier, or hint and fails if it finds
zero or more than one visible enabled target. The app must be foreground.

`bundle_id` remains accepted as a compatibility alias for Apple callers. New
cross-platform clients should send `app_id`; passing different `app_id` and
`bundle_id` values is invalid.
