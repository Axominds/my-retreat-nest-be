use axum::http::HeaderMap;

/// Subdomains that are never treated as retreat tenants.
pub const RESERVED_SUBDOMAINS: &[&str] = &[
    "www", "api", "admin", "app", "static", "mail", "_next",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantResolveError {
    /// No usable host information in headers.
    Missing,
    /// Host present but malformed (e.g. deep subdomain, bad candidate shape,
    /// or host unrelated to the root domain).
    Invalid,
    /// Apex domain, reserved subdomain, or otherwise not a tenant.
    /// Callers map this to 404 so the frontend redirects to the apex.
    NonTenant,
}

/// Extracts the bare host from an `Origin`/`Referer` header value.
/// Returns `None` when the value is not a parseable http(s) URL.
fn host_from_url_value(value: &str) -> Option<String> {
    let value: &str = value.trim();
    let rest: &str = value
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(value);
    let host: &str = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let host: &str = host.split('@').next_back().unwrap_or(host);
    // Strip port if present.
    let host: &str = host.split(':').next().unwrap_or(host);
    let host: String = host.trim().trim_end_matches('.').to_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(host)
}

fn normalize_root(root: &str) -> String {
    let lowered: String = root.trim().trim_end_matches('.').to_lowercase();
    lowered
        .strip_prefix("www.")
        .map(|s| s.to_string())
        .unwrap_or(lowered)
}

fn is_valid_candidate(candidate: &str) -> bool {
    if candidate.is_empty() || candidate.len() > 63 {
        return false;
    }
    let bytes: &[u8] = candidate.as_bytes();
    if bytes[0] == b'-' || bytes[bytes.len() - 1] == b'-' {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
}

/// Resolves a retreat slug (subdomain) purely from request headers.
///
/// Priority: `Origin` -> `Referer` -> `Host`/`X-Forwarded-Host`.
/// `root_domain` is the apex (e.g. `myretreatnest.com`, or `localhost` for dev).
pub fn resolve_tenant_slug(
    headers: &HeaderMap,
    root_domain: &str,
) -> Result<String, TenantResolveError> {
    let host: Option<String> = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .and_then(host_from_url_value)
        .or_else(|| {
            headers
                .get("referer")
                .and_then(|v| v.to_str().ok())
                .and_then(host_from_url_value)
        })
        .or_else(|| {
            headers
                .get("x-forwarded-host")
                .and_then(|v| v.to_str().ok())
                .map(|v| {
                    v.split(',')
                        .next()
                        .unwrap_or(v)
                        .trim()
                        .trim_end_matches('.')
                        .to_lowercase()
                })
                .and_then(|v| {
                    let no_port: &str = v.split(':').next().unwrap_or(&v);
                    if no_port.is_empty() {
                        None
                    } else {
                        Some(no_port.to_string())
                    }
                })
        })
        .or_else(|| {
            headers
                .get("host")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.trim().trim_end_matches('.').to_lowercase())
                .and_then(|v| {
                    let no_port: &str = v.split(':').next().unwrap_or(&v);
                    if no_port.is_empty() {
                        None
                    } else {
                        Some(no_port.to_string())
                    }
                })
        });

    let host: String = match host {
        Some(h) => h,
        None => return Err(TenantResolveError::Missing),
    };

    let root: String = normalize_root(root_domain);
    if root.is_empty() {
        return Err(TenantResolveError::Invalid);
    }

    // Apex (and www-apex) is never a tenant.
    if host == root || host == format!("www.{root}") {
        return Err(TenantResolveError::NonTenant);
    }

    let suffix: String = format!(".{root}");
    let candidate: &str = match host.strip_suffix(suffix.as_str()) {
        Some(left) => left,
        None => {
            // Host unrelated to the root domain (direct IP, foreign domain...).
            // Single-label `localhost` dev apex:
            if host == "localhost" {
                return Err(TenantResolveError::NonTenant);
            }
            return Err(TenantResolveError::Invalid);
        }
    };

    if candidate.is_empty() || candidate.contains('.') {
        return Err(TenantResolveError::NonTenant);
    }

    if RESERVED_SUBDOMAINS.contains(&candidate) {
        return Err(TenantResolveError::NonTenant);
    }

    if !is_valid_candidate(candidate) {
        return Err(TenantResolveError::Invalid);
    }

    Ok(candidate.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                k.parse::<HeaderName>().unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        map
    }

    #[test]
    fn origin_subdomain_resolves() {
        let h = headers(&[("origin", "https://xyz.myretreatnest.com")]);
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Ok("xyz".to_string())
        );
    }

    #[test]
    fn origin_takes_priority_over_host() {
        let h = headers(&[
            ("origin", "https://aaa.myretreatnest.com"),
            ("host", "bbb.myretreatnest.com"),
        ]);
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Ok("aaa".to_string())
        );
    }

    #[test]
    fn falls_back_to_referer_then_host() {
        let h = headers(&[("referer", "https://xyz.myretreatnest.com/some/page")]);
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Ok("xyz".to_string())
        );
        let h = headers(&[("host", "xyz.myretreatnest.com")]);
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Ok("xyz".to_string())
        );
    }

    #[test]
    fn forwarded_host_supported_with_port_and_case() {
        let h = headers(&[("x-forwarded-host", "XYZ.myretreatnest.com:443")]);
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Ok("xyz".to_string())
        );
    }

    #[test]
    fn apex_and_reserved_are_non_tenant() {
        for host in [
            "myretreatnest.com",
            "www.myretreatnest.com",
            "admin.myretreatnest.com",
            "api.myretreatnest.com",
        ] {
            let h = headers(&[("host", host)]);
            assert_eq!(
                resolve_tenant_slug(&h, "myretreatnest.com"),
                Err(TenantResolveError::NonTenant),
                "host: {host}"
            );
        }
    }

    #[test]
    fn deep_subdomain_is_non_tenant() {
        let h = headers(&[("host", "a.b.myretreatnest.com")]);
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Err(TenantResolveError::NonTenant)
        );
    }

    #[test]
    fn localhost_variants() {
        let h = headers(&[("host", "xyz.localhost:3000")]);
        assert_eq!(resolve_tenant_slug(&h, "localhost"), Ok("xyz".to_string()));
        let h = headers(&[("host", "localhost:3000")]);
        assert_eq!(
            resolve_tenant_slug(&h, "localhost"),
            Err(TenantResolveError::NonTenant)
        );
    }

    #[test]
    fn missing_headers_and_foreign_hosts() {
        let h = HeaderMap::new();
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Err(TenantResolveError::Missing)
        );
        let h = headers(&[("host", "evil.com")]);
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Err(TenantResolveError::Invalid)
        );
        let h = headers(&[("host", "-bad-.myretreatnest.com")]);
        assert_eq!(
            resolve_tenant_slug(&h, "myretreatnest.com"),
            Err(TenantResolveError::Invalid)
        );
    }
}
