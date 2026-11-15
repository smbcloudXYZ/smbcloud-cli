fn main() {
    eprintln!("smbpndk-cli has been renamed to smbcloud-cli.");
    eprintln!();
    eprintln!("Install the new crate instead:");
    eprintln!();
    eprintln!("    cargo install smbcloud-cli");
    eprintln!("    smb --help");
    eprintln!();
    eprintln!("The binary is still called `smb`. See https://smbcloud.xyz for details.");
    std::process::exit(1);
}
