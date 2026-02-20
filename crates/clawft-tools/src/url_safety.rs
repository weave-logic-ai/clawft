//! URL safety validation for SSRF protection.
//!
//! Provides [`validate_url`] to check URLs against private IP ranges,
//! cloud metadata endpoints, and configurable domain allow/block lists.
//! This prevents Server-Side Request Forgery (SSRF) attacks where an
//! attacker tricks the application into making requests to internal
//! services or cloud instance metadata endpoints.
//!
//! The [`UrlPolicy`] type is defined in [`clawft_types::security`] and
//! re-exported here for convenience.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

use ipnet::{Ipv4Net, Ipv6Net};
use url::Url;

// Re-export the canonical UrlPolicy from clawft-types.
pub use clawft_types::security::UrlPolicy;

/// Cloud metadata service hostnames that must always be blocked.
const METADATA_HOSTS: &[&str] = &[
    "169.254.169.254",
    "metadata.google.internal",
    "metadata.internal",
];

/// Private and reserved IPv4 networks.
const BLOCKED_IPV4_CIDRS: &[&str] = &[
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "127.0.0.0/8",
    "169.254.0.0/16",
    "0.0.0.0/8",
];

/// Private and reserved IPv6 networks.
const BLOCKED_IPV6_CIDRS: &[&str] = &["::1/128", "fe80::/10", "fc00::/7"];

/// Errors returned by URL safety validation.
#[derive(Debug, thiserror::Error)]
pub enum UrlSafetyError {
    /// The URL could not be parsed or is structurally invalid.
    #[error("invalid URL '{url}': {reason}")]
    InvalidUrl { url: String, reason: String },

    /// The URL targets a cloud metadata endpoint.
    #[error("blocked cloud metadata endpoint: {host}")]
    MetadataEndpoint { host: String },

    /// The URL resolves to a private/reserved IP address.
    #[error("blocked private IP {ip} for host {host}")]
    PrivateIp { ip: String, host: String },

    /// The URL targets an explicitly blocked domain.
    #[error("blocked domain: {host}")]
    BlockedDomain { host: String },

    /// DNS resolution failed for the host.
    #[error("failed to resolve host '{host}': {reason}")]
    ResolutionFailed { host: String, reason: String },
}

/// Check whether an IP address belongs to a blocked private/reserved range.
///
/// Returns `true` if the IP is in any of the standard private, loopback,
/// link-local, or reserved networks. For IPv4-mapped IPv6 addresses, the
/// embedded IPv4 address is also checked.
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => {
            // Check IPv6 reserved ranges.
            if is_blocked_ipv6(v6) {
                return true;
            }
            // Check IPv4-mapped IPv6 addresses (::ffff:x.x.x.x).
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_blocked_ipv4(mapped);
            }
            false
        }
    }
}

/// Check an IPv4 address against blocked CIDR ranges.
fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    BLOCKED_IPV4_CIDRS
        .iter()
        .filter_map(|cidr| cidr.parse::<Ipv4Net>().ok())
        .any(|net| net.contains(&ip))
}

/// Check an IPv6 address against blocked CIDR ranges.
fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    BLOCKED_IPV6_CIDRS
        .iter()
        .filter_map(|cidr| cidr.parse::<Ipv6Net>().ok())
        .any(|net| net.contains(&ip))
}

/// Validate a URL against the given safety policy.
///
/// Checks the URL for SSRF risks including:
/// - Cloud metadata endpoints (e.g. `169.254.169.254`)
/// - Private/reserved IP address ranges
/// - Explicitly blocked domains
/// - DNS resolution to private IPs
///
/// If the host is in `policy.allowed_domains`, all checks are bypassed.
/// If `policy.enabled` is `false`, the function always returns `Ok(())`.
///
/// # Examples
///
/// ```
/// use clawft_tools::url_safety::{validate_url, UrlPolicy};
///
/// let policy = UrlPolicy::default();
/// assert!(validate_url("https://example.com", &policy).is_ok());
/// assert!(validate_url("http://169.254.169.254/latest/", &policy).is_err());
/// ```
pub fn validate_url(url_str: &str, policy: &UrlPolicy) -> Result<(), UrlSafetyError> {
    if !policy.enabled {
        return Ok(());
    }

    let parsed = Url::parse(url_str).map_err(|e| UrlSafetyError::InvalidUrl {
        url: url_str.to_string(),
        reason: e.to_string(),
    })?;

    let host = parsed
        .host_str()
        .ok_or_else(|| UrlSafetyError::InvalidUrl {
            url: url_str.to_string(),
            reason: "URL has no host".to_string(),
        })?
        .to_string();

    // Allowed domains bypass all other checks.
    if policy.allowed_domains.contains(&host) {
        return Ok(());
    }

    // Check blocked domains.
    if policy.blocked_domains.contains(&host) {
        return Err(UrlSafetyError::BlockedDomain { host: host.clone() });
    }

    // Check cloud metadata endpoints.
    if METADATA_HOSTS.iter().any(|&m| m == host) {
        return Err(UrlSafetyError::MetadataEndpoint { host: host.clone() });
    }

    // If private IPs are allowed, skip IP-based checks.
    if policy.allow_private {
        return Ok(());
    }

    // Try to parse the host directly as an IP address.
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(UrlSafetyError::PrivateIp {
                ip: ip.to_string(),
                host: host.clone(),
            });
        }
        return Ok(());
    }

    // Host is a domain name; attempt DNS resolution (synchronous, best-effort).
    let port = parsed.port_or_known_default().unwrap_or(80);
    let socket_addr = format!("{host}:{port}");
    match socket_addr.to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                if is_blocked_ip(addr.ip()) {
                    return Err(UrlSafetyError::PrivateIp {
                        ip: addr.ip().to_string(),
                        host: host.clone(),
                    });
                }
            }
        }
        Err(_) => {
            // DNS resolution failed. We already checked metadata hosts
            // and blocked domains above, so we allow the URL through.
            // A downstream HTTP client will fail if the host truly
            // doesn't resolve.
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn default_policy() -> UrlPolicy {
        UrlPolicy::default()
    }

    // --- Public URLs should pass ---

    #[test]
    fn public_url_example_com() {
        assert!(validate_url("https://example.com", &default_policy()).is_ok());
    }

    #[test]
    fn public_url_openai_api() {
        assert!(validate_url("https://api.openai.com/v1/chat", &default_policy()).is_ok());
    }

    #[test]
    fn public_url_with_path_and_query() {
        assert!(validate_url("https://example.com/path?key=value", &default_policy()).is_ok());
    }

    // --- Private IPv4 blocked ---

    #[test]
    fn private_ip_10_network() {
        let err = validate_url("http://10.0.0.1", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    #[test]
    fn private_ip_172_16_network() {
        let err = validate_url("http://172.16.0.1", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    #[test]
    fn private_ip_192_168_network() {
        let err = validate_url("http://192.168.1.1", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    // --- Loopback blocked ---

    #[test]
    fn loopback_127_0_0_1() {
        let err = validate_url("http://127.0.0.1", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    #[test]
    fn loopback_127_0_0_2() {
        let err = validate_url("http://127.0.0.2", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    // --- Link-local blocked ---

    #[test]
    fn link_local_169_254_1_1() {
        let err = validate_url("http://169.254.1.1", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    // --- Cloud metadata endpoints blocked ---

    #[test]
    fn metadata_aws() {
        let err = validate_url(
            "http://169.254.169.254/latest/meta-data/",
            &default_policy(),
        )
        .unwrap_err();
        assert!(matches!(err, UrlSafetyError::MetadataEndpoint { .. }));
    }

    #[test]
    fn metadata_gcp() {
        let err = validate_url("http://metadata.google.internal/", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::MetadataEndpoint { .. }));
    }

    // --- IPv6 blocked ---

    #[test]
    fn ipv6_loopback() {
        let err = validate_url("http://[::1]/", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    #[test]
    fn ipv6_link_local() {
        let err = validate_url("http://[fe80::1]/", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    #[test]
    fn ipv6_unique_local() {
        let err = validate_url("http://[fd00::1]/", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    #[test]
    fn ipv4_mapped_ipv6() {
        let err = validate_url("http://[::ffff:10.0.0.1]/", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    // --- Allowed domains bypass checks ---

    #[test]
    fn allowed_domain_bypasses_checks() {
        let policy = UrlPolicy::new(
            true,
            false,
            HashSet::from(["internal.corp".to_string()]),
            HashSet::new(),
        );
        assert!(validate_url("http://internal.corp/api", &policy).is_ok());
    }

    // --- Blocked domains ---

    #[test]
    fn blocked_domain() {
        let policy = UrlPolicy::new(
            true,
            false,
            HashSet::new(),
            HashSet::from(["evil.com".to_string()]),
        );
        let err = validate_url("http://evil.com/", &policy).unwrap_err();
        assert!(matches!(err, UrlSafetyError::BlockedDomain { .. }));
    }

    // --- Policy disabled ---

    #[test]
    fn policy_disabled_allows_everything() {
        let policy = UrlPolicy::permissive();
        assert!(validate_url("http://127.0.0.1", &policy).is_ok());
        assert!(validate_url("http://169.254.169.254/latest/", &policy).is_ok());
    }

    // --- Invalid URL ---

    #[test]
    fn invalid_url() {
        let err = validate_url("not-a-url", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::InvalidUrl { .. }));
    }

    // --- Zero network blocked ---

    #[test]
    fn zero_network_blocked() {
        let err = validate_url("http://0.0.0.1", &default_policy()).unwrap_err();
        assert!(matches!(err, UrlSafetyError::PrivateIp { .. }));
    }

    // --- Direct is_blocked_ip tests ---

    #[test]
    fn is_blocked_ip_v4_private() {
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 5, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
        assert!(is_blocked_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 1))));
    }

    #[test]
    fn is_blocked_ip_v4_public() {
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_blocked_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
    }

    #[test]
    fn is_blocked_ip_v6_reserved() {
        assert!(is_blocked_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(is_blocked_ip(IpAddr::V6(
            "fe80::1".parse::<Ipv6Addr>().unwrap()
        )));
        assert!(is_blocked_ip(IpAddr::V6(
            "fd00::1".parse::<Ipv6Addr>().unwrap()
        )));
    }

    #[test]
    fn is_blocked_ip_v6_public() {
        assert!(!is_blocked_ip(IpAddr::V6(
            "2001:4860:4860::8888".parse::<Ipv6Addr>().unwrap()
        )));
    }

    #[test]
    fn is_blocked_ip_v4_mapped_v6() {
        // ::ffff:10.0.0.1 should be blocked because 10.0.0.1 is private.
        let mapped: Ipv6Addr = "::ffff:10.0.0.1".parse().unwrap();
        assert!(is_blocked_ip(IpAddr::V6(mapped)));
    }

    #[test]
    fn is_blocked_ip_v4_mapped_v6_public() {
        // ::ffff:8.8.8.8 should not be blocked.
        let mapped: Ipv6Addr = "::ffff:8.8.8.8".parse().unwrap();
        assert!(!is_blocked_ip(IpAddr::V6(mapped)));
    }

    // --- Allow-private policy ---

    #[test]
    fn allow_private_permits_private_ips() {
        let policy = UrlPolicy::new(true, true, HashSet::new(), HashSet::new());
        assert!(validate_url("http://10.0.0.1/api", &policy).is_ok());
        assert!(validate_url("http://192.168.1.1/api", &policy).is_ok());
    }

    // --- UrlPolicy constructors ---

    #[test]
    fn default_policy_is_enabled() {
        let policy = UrlPolicy::default();
        assert!(policy.enabled);
        assert!(!policy.allow_private);
        assert!(policy.allowed_domains.is_empty());
        assert!(policy.blocked_domains.is_empty());
    }

    #[test]
    fn permissive_policy_is_disabled() {
        let policy = UrlPolicy::permissive();
        assert!(!policy.enabled);
    }
}
