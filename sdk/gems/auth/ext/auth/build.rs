fn main() {
    // On macOS, a Rust `cdylib` is emitted with an empty LC_ID_DYLIB install
    // name. The dyld in macOS 15+/Tahoe (Darwin 27) validates this strictly and
    // refuses to load the resulting `.bundle` with:
    //   "load command #4 string extends beyond end of load command"
    // Give the dylib a valid install name at link time so the Ruby extension
    // loads on the stricter loader.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/auth.bundle");
    }
}
