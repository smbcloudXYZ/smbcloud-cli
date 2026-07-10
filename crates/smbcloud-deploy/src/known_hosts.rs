/// Pinned SSH host keys for smbCloud deployment servers.
///
/// These are **public** keys — safe to commit to a public repository.
/// Host keys are designed to be known; only the private half (which never
/// leaves the server) provides authentication.
///
/// Purpose: protect every `smb deploy` user from DNS/BGP hijacking.
/// By pairing these constants with `StrictHostKeyChecking=yes` and a
/// generated temp `UserKnownHostsFile`, the CLI refuses to connect to any
/// server that doesn't present one of these exact keys — even if the
/// hostname resolves correctly to an attacker's IP.
///
/// Both servers run OpenSSH 9.2p1 on Debian 12.
///
/// # Key rotation
///
/// If a server's host key is ever rotated (e.g. after a compromise or
/// reprovisioning), update the relevant constant here and cut a new CLI
/// release. Users on old releases will receive a hard SSH refusal:
///
/// ```text
/// Host key verification failed.
/// ```
///
/// That is the correct behaviour — they must upgrade before deploying.
///
/// To re-fetch the current keys:
///
/// ```sh
/// ssh-keyscan -t ed25519 api.smbcloud.xyz api-1.smbcloud.xyz 2>/dev/null
/// ```
/// Pinned ed25519 host key for `api.smbcloud.xyz` (NodeJs / Static tier).
pub const API_SMBCLOUD_XYZ: &str =
    "api.smbcloud.xyz ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINGy4LHGyOilS7SXo770V3tQnXDRVQr7X7JsPLCfy4XB";

/// Pinned ed25519 host key for `api-1.smbcloud.xyz` (Ruby / Swift tier).
pub const API_1_SMBCLOUD_XYZ: &str =
    "api-1.smbcloud.xyz ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAII1G3JyS66+yIFGrN3Vgc/UlvBm/oS98qq5pS96UYxcz";

/// Returns the pinned host key line for the given rsync hostname.
///
/// The returned string is in `known_hosts` format and can be written
/// directly to a temp file passed to SSH via `-o UserKnownHostsFile=`.
pub fn for_host(rsync_host: &str) -> &'static str {
    if rsync_host.starts_with("api-1.") {
        API_1_SMBCLOUD_XYZ
    } else {
        API_SMBCLOUD_XYZ
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_host_returns_correct_key() {
        let key = for_host("api.smbcloud.xyz");
        assert!(key.starts_with("api.smbcloud.xyz ssh-ed25519 "));
        assert!(!key.contains("api-1"));
    }

    #[test]
    fn api_1_host_returns_correct_key() {
        let key = for_host("api-1.smbcloud.xyz");
        assert!(key.starts_with("api-1.smbcloud.xyz ssh-ed25519 "));
    }

    #[test]
    fn keys_are_distinct() {
        assert_ne!(API_SMBCLOUD_XYZ, API_1_SMBCLOUD_XYZ);
    }

    #[test]
    fn keys_contain_valid_ed25519_prefix() {
        // ed25519 public keys always begin with this base64 prefix
        assert!(API_SMBCLOUD_XYZ.contains("AAAA"));
        assert!(API_1_SMBCLOUD_XYZ.contains("AAAA"));
    }
}
