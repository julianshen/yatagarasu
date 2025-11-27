// IP filtering module - allowlist and blocklist with CIDR support
// Phase 35: Advanced Security

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// IP filtering configuration for a bucket
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IpFilterConfig {
    /// List of allowed IPs or CIDR ranges (if set, only these are allowed)
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// List of blocked IPs or CIDR ranges
    #[serde(default)]
    pub blocklist: Vec<String>,
}

/// Parsed IP filter with compiled CIDR ranges
#[derive(Debug, Clone)]
pub struct IpFilter {
    allowlist: Vec<IpRange>,
    blocklist: Vec<IpRange>,
}

/// Represents an IP range (single IP or CIDR network)
#[derive(Debug, Clone)]
pub enum IpRange {
    Single(IpAddr),
    CidrV4 { network: Ipv4Addr, prefix_len: u8 },
    CidrV6 { network: Ipv6Addr, prefix_len: u8 },
}

impl IpRange {
    /// Parse an IP or CIDR string
    pub fn parse(s: &str) -> Result<Self, IpFilterError> {
        let s = s.trim();

        // Check if it's a CIDR notation
        if let Some(idx) = s.find('/') {
            let (ip_str, prefix_str) = s.split_at(idx);
            let prefix_str = &prefix_str[1..]; // Skip the '/'

            let prefix_len: u8 = prefix_str
                .parse()
                .map_err(|_| IpFilterError::InvalidCidr(s.to_string()))?;

            // Parse as IPv4 or IPv6
            if let Ok(ipv4) = Ipv4Addr::from_str(ip_str) {
                if prefix_len > 32 {
                    return Err(IpFilterError::InvalidCidr(s.to_string()));
                }
                // Normalize the network address
                let network = Self::normalize_ipv4(ipv4, prefix_len);
                Ok(IpRange::CidrV4 {
                    network,
                    prefix_len,
                })
            } else if let Ok(ipv6) = Ipv6Addr::from_str(ip_str) {
                if prefix_len > 128 {
                    return Err(IpFilterError::InvalidCidr(s.to_string()));
                }
                let network = Self::normalize_ipv6(ipv6, prefix_len);
                Ok(IpRange::CidrV6 {
                    network,
                    prefix_len,
                })
            } else {
                Err(IpFilterError::InvalidIp(ip_str.to_string()))
            }
        } else {
            // Single IP address
            let ip = IpAddr::from_str(s).map_err(|_| IpFilterError::InvalidIp(s.to_string()))?;
            Ok(IpRange::Single(ip))
        }
    }

    /// Normalize an IPv4 address to its network address
    fn normalize_ipv4(ip: Ipv4Addr, prefix_len: u8) -> Ipv4Addr {
        let bits = u32::from(ip);
        let mask = if prefix_len == 0 {
            0
        } else {
            !0u32 << (32 - prefix_len)
        };
        Ipv4Addr::from(bits & mask)
    }

    /// Normalize an IPv6 address to its network address
    fn normalize_ipv6(ip: Ipv6Addr, prefix_len: u8) -> Ipv6Addr {
        let bits = u128::from(ip);
        let mask = if prefix_len == 0 {
            0
        } else {
            !0u128 << (128 - prefix_len)
        };
        Ipv6Addr::from(bits & mask)
    }

    /// Check if an IP address matches this range
    pub fn contains(&self, ip: &IpAddr) -> bool {
        match (self, ip) {
            (IpRange::Single(range_ip), ip) => range_ip == ip,

            (
                IpRange::CidrV4 {
                    network,
                    prefix_len,
                },
                IpAddr::V4(ipv4),
            ) => {
                let ip_bits = u32::from(*ipv4);
                let network_bits = u32::from(*network);
                let mask = if *prefix_len == 0 {
                    0
                } else {
                    !0u32 << (32 - prefix_len)
                };
                (ip_bits & mask) == network_bits
            }

            (
                IpRange::CidrV6 {
                    network,
                    prefix_len,
                },
                IpAddr::V6(ipv6),
            ) => {
                let ip_bits = u128::from(*ipv6);
                let network_bits = u128::from(*network);
                let mask = if *prefix_len == 0 {
                    0
                } else {
                    !0u128 << (128 - prefix_len)
                };
                (ip_bits & mask) == network_bits
            }

            // IPv4 CIDR doesn't match IPv6 address and vice versa
            (IpRange::CidrV4 { .. }, IpAddr::V6(_)) => false,
            (IpRange::CidrV6 { .. }, IpAddr::V4(_)) => false,
        }
    }
}

impl IpFilter {
    /// Create a new IP filter from configuration
    pub fn new(config: &IpFilterConfig) -> Result<Self, IpFilterError> {
        let allowlist = config
            .allowlist
            .iter()
            .map(|s| IpRange::parse(s))
            .collect::<Result<Vec<_>, _>>()?;

        let blocklist = config
            .blocklist
            .iter()
            .map(|s| IpRange::parse(s))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            allowlist,
            blocklist,
        })
    }

    /// Create an empty filter (allows all)
    pub fn allow_all() -> Self {
        Self {
            allowlist: vec![],
            blocklist: vec![],
        }
    }

    /// Check if an IP address is allowed
    ///
    /// Rules:
    /// 1. If allowlist is set, IP must be in allowlist
    /// 2. If IP is in blocklist, it's rejected (unless in allowlist)
    /// 3. Allowlist takes precedence over blocklist
    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        // If allowlist is configured, check if IP is in it
        if !self.allowlist.is_empty() {
            // Allowlist takes precedence - if in allowlist, always allow
            for range in &self.allowlist {
                if range.contains(ip) {
                    return true;
                }
            }
            // Not in allowlist, reject
            return false;
        }

        // No allowlist - check blocklist
        for range in &self.blocklist {
            if range.contains(ip) {
                return false;
            }
        }

        // Not blocked, allow
        true
    }

    /// Check if an IP string is allowed
    pub fn is_allowed_str(&self, ip_str: &str) -> Result<bool, IpFilterError> {
        let ip =
            IpAddr::from_str(ip_str).map_err(|_| IpFilterError::InvalidIp(ip_str.to_string()))?;
        Ok(self.is_allowed(&ip))
    }

    /// Check if filter is configured (has any rules)
    pub fn is_configured(&self) -> bool {
        !self.allowlist.is_empty() || !self.blocklist.is_empty()
    }

    /// Check if filter has allowlist rules
    pub fn has_allowlist(&self) -> bool {
        !self.allowlist.is_empty()
    }

    /// Check if filter has blocklist rules
    pub fn has_blocklist(&self) -> bool {
        !self.blocklist.is_empty()
    }
}

/// IP filter errors
#[derive(Debug, thiserror::Error)]
pub enum IpFilterError {
    #[error("Invalid IP address: {0}")]
    InvalidIp(String),

    #[error("Invalid CIDR notation: {0}")]
    InvalidCidr(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // IpRange parsing tests
    // ============================================================

    #[test]
    fn test_parse_single_ipv4() {
        let range = IpRange::parse("192.168.1.1").unwrap();
        assert!(matches!(range, IpRange::Single(IpAddr::V4(_))));
    }

    #[test]
    fn test_parse_single_ipv6() {
        let range = IpRange::parse("::1").unwrap();
        assert!(matches!(range, IpRange::Single(IpAddr::V6(_))));
    }

    #[test]
    fn test_parse_cidr_ipv4() {
        let range = IpRange::parse("192.168.0.0/24").unwrap();
        match range {
            IpRange::CidrV4 {
                network,
                prefix_len,
            } => {
                assert_eq!(network, Ipv4Addr::new(192, 168, 0, 0));
                assert_eq!(prefix_len, 24);
            }
            _ => panic!("Expected CidrV4"),
        }
    }

    #[test]
    fn test_parse_cidr_ipv6() {
        let range = IpRange::parse("2001:db8::/32").unwrap();
        match range {
            IpRange::CidrV6 { prefix_len, .. } => {
                assert_eq!(prefix_len, 32);
            }
            _ => panic!("Expected CidrV6"),
        }
    }

    #[test]
    fn test_parse_cidr_normalizes_network() {
        // 192.168.1.100/24 should normalize to 192.168.1.0/24
        let range = IpRange::parse("192.168.1.100/24").unwrap();
        match range {
            IpRange::CidrV4 {
                network,
                prefix_len,
            } => {
                assert_eq!(network, Ipv4Addr::new(192, 168, 1, 0));
                assert_eq!(prefix_len, 24);
            }
            _ => panic!("Expected CidrV4"),
        }
    }

    #[test]
    fn test_parse_invalid_ip() {
        assert!(IpRange::parse("not-an-ip").is_err());
    }

    #[test]
    fn test_parse_invalid_cidr_prefix() {
        assert!(IpRange::parse("192.168.0.0/33").is_err()); // IPv4 max is 32
        assert!(IpRange::parse("::1/129").is_err()); // IPv6 max is 128
    }

    // ============================================================
    // IpRange matching tests
    // ============================================================

    #[test]
    fn test_single_ip_matches_exact() {
        let range = IpRange::parse("192.168.1.1").unwrap();
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(range.contains(&ip));
    }

    #[test]
    fn test_single_ip_no_match() {
        let range = IpRange::parse("192.168.1.1").unwrap();
        let ip: IpAddr = "192.168.1.2".parse().unwrap();
        assert!(!range.contains(&ip));
    }

    #[test]
    fn test_cidr_ipv4_matches_in_range() {
        let range = IpRange::parse("192.168.1.0/24").unwrap();

        // Should match
        assert!(range.contains(&"192.168.1.0".parse().unwrap()));
        assert!(range.contains(&"192.168.1.1".parse().unwrap()));
        assert!(range.contains(&"192.168.1.255".parse().unwrap()));

        // Should not match
        assert!(!range.contains(&"192.168.0.1".parse().unwrap()));
        assert!(!range.contains(&"192.168.2.1".parse().unwrap()));
        assert!(!range.contains(&"10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn test_cidr_ipv4_16_prefix() {
        let range = IpRange::parse("10.0.0.0/16").unwrap();

        assert!(range.contains(&"10.0.0.1".parse().unwrap()));
        assert!(range.contains(&"10.0.255.255".parse().unwrap()));
        assert!(!range.contains(&"10.1.0.1".parse().unwrap()));
    }

    #[test]
    fn test_cidr_ipv4_8_prefix() {
        let range = IpRange::parse("10.0.0.0/8").unwrap();

        assert!(range.contains(&"10.0.0.1".parse().unwrap()));
        assert!(range.contains(&"10.255.255.255".parse().unwrap()));
        assert!(!range.contains(&"11.0.0.1".parse().unwrap()));
    }

    #[test]
    fn test_cidr_ipv6_matches() {
        let range = IpRange::parse("2001:db8::/32").unwrap();

        assert!(range.contains(&"2001:db8::1".parse().unwrap()));
        assert!(range.contains(&"2001:db8:ffff:ffff::1".parse().unwrap()));
        assert!(!range.contains(&"2001:db9::1".parse().unwrap()));
    }

    #[test]
    fn test_cidr_ipv4_no_match_ipv6() {
        let range = IpRange::parse("192.168.0.0/24").unwrap();
        let ipv6: IpAddr = "::1".parse().unwrap();
        assert!(!range.contains(&ipv6));
    }

    // ============================================================
    // IpFilter tests
    // ============================================================

    #[test]
    fn test_ip_filter_allow_all_by_default() {
        let config = IpFilterConfig::default();
        let filter = IpFilter::new(&config).unwrap();

        assert!(filter.is_allowed(&"192.168.1.1".parse().unwrap()));
        assert!(filter.is_allowed(&"10.0.0.1".parse().unwrap()));
        assert!(filter.is_allowed(&"::1".parse().unwrap()));
    }

    #[test]
    fn test_ip_filter_blocklist_rejects() {
        let config = IpFilterConfig {
            allowlist: vec![],
            blocklist: vec!["192.168.1.100".to_string(), "10.0.0.0/8".to_string()],
        };
        let filter = IpFilter::new(&config).unwrap();

        // Blocked IPs
        assert!(!filter.is_allowed(&"192.168.1.100".parse().unwrap()));
        assert!(!filter.is_allowed(&"10.0.0.1".parse().unwrap()));
        assert!(!filter.is_allowed(&"10.255.255.255".parse().unwrap()));

        // Allowed IPs
        assert!(filter.is_allowed(&"192.168.1.1".parse().unwrap()));
        assert!(filter.is_allowed(&"172.16.0.1".parse().unwrap()));
    }

    #[test]
    fn test_ip_filter_allowlist_only_allows_listed() {
        let config = IpFilterConfig {
            allowlist: vec!["192.168.1.0/24".to_string(), "10.0.0.1".to_string()],
            blocklist: vec![],
        };
        let filter = IpFilter::new(&config).unwrap();

        // Allowed IPs
        assert!(filter.is_allowed(&"192.168.1.1".parse().unwrap()));
        assert!(filter.is_allowed(&"192.168.1.255".parse().unwrap()));
        assert!(filter.is_allowed(&"10.0.0.1".parse().unwrap()));

        // Not in allowlist - rejected
        assert!(!filter.is_allowed(&"192.168.2.1".parse().unwrap()));
        assert!(!filter.is_allowed(&"10.0.0.2".parse().unwrap()));
        assert!(!filter.is_allowed(&"172.16.0.1".parse().unwrap()));
    }

    #[test]
    fn test_ip_filter_allowlist_takes_precedence() {
        let config = IpFilterConfig {
            allowlist: vec!["192.168.1.100".to_string()],
            blocklist: vec!["192.168.1.0/24".to_string()],
        };
        let filter = IpFilter::new(&config).unwrap();

        // In both allowlist and blocklist - allowlist wins
        assert!(filter.is_allowed(&"192.168.1.100".parse().unwrap()));

        // Not in allowlist, even though not in blocklist - rejected (allowlist is exclusive)
        assert!(!filter.is_allowed(&"10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn test_ip_filter_is_allowed_str() {
        let config = IpFilterConfig {
            allowlist: vec![],
            blocklist: vec!["192.168.1.100".to_string()],
        };
        let filter = IpFilter::new(&config).unwrap();

        assert!(!filter.is_allowed_str("192.168.1.100").unwrap());
        assert!(filter.is_allowed_str("192.168.1.1").unwrap());
        assert!(filter.is_allowed_str("invalid-ip").is_err());
    }

    #[test]
    fn test_ip_filter_is_configured() {
        let empty = IpFilter::allow_all();
        assert!(!empty.is_configured());

        let with_allowlist = IpFilter::new(&IpFilterConfig {
            allowlist: vec!["10.0.0.1".to_string()],
            blocklist: vec![],
        })
        .unwrap();
        assert!(with_allowlist.is_configured());
        assert!(with_allowlist.has_allowlist());
        assert!(!with_allowlist.has_blocklist());

        let with_blocklist = IpFilter::new(&IpFilterConfig {
            allowlist: vec![],
            blocklist: vec!["10.0.0.1".to_string()],
        })
        .unwrap();
        assert!(with_blocklist.is_configured());
        assert!(!with_blocklist.has_allowlist());
        assert!(with_blocklist.has_blocklist());
    }

    #[test]
    fn test_ip_filter_config_deserialize() {
        let yaml = r#"
allowlist:
  - "192.168.1.0/24"
  - "10.0.0.1"
blocklist:
  - "172.16.0.0/12"
"#;
        let config: IpFilterConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.allowlist.len(), 2);
        assert_eq!(config.blocklist.len(), 1);
    }

    #[test]
    fn test_ip_filter_config_default() {
        let config = IpFilterConfig::default();
        assert!(config.allowlist.is_empty());
        assert!(config.blocklist.is_empty());
    }

    #[test]
    fn test_ip_filter_error_display() {
        let err = IpFilterError::InvalidIp("bad-ip".to_string());
        assert!(err.to_string().contains("Invalid IP"));

        let err = IpFilterError::InvalidCidr("bad/cidr".to_string());
        assert!(err.to_string().contains("Invalid CIDR"));
    }

    // ============================================================
    // Edge cases
    // ============================================================

    #[test]
    fn test_cidr_zero_prefix() {
        // /0 matches everything
        let range = IpRange::parse("0.0.0.0/0").unwrap();
        assert!(range.contains(&"0.0.0.0".parse().unwrap()));
        assert!(range.contains(&"255.255.255.255".parse().unwrap()));
        assert!(range.contains(&"192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn test_cidr_32_prefix() {
        // /32 matches exactly one IP
        let range = IpRange::parse("192.168.1.1/32").unwrap();
        assert!(range.contains(&"192.168.1.1".parse().unwrap()));
        assert!(!range.contains(&"192.168.1.2".parse().unwrap()));
    }

    #[test]
    fn test_cidr_128_prefix_ipv6() {
        // /128 matches exactly one IPv6
        let range = IpRange::parse("::1/128").unwrap();
        assert!(range.contains(&"::1".parse().unwrap()));
        assert!(!range.contains(&"::2".parse().unwrap()));
    }

    #[test]
    fn test_whitespace_trimmed() {
        let range = IpRange::parse("  192.168.1.1  ").unwrap();
        assert!(range.contains(&"192.168.1.1".parse().unwrap()));
    }
}
