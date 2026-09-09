//! Public http(s) fetch with SSRF guards for LLM tools.

use std::net::IpAddr;
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::header;
use tokio::net::lookup_host;

const MAX_REDIRECTS: u32 = 5;

pub struct FetchOptions<'a> {
    pub user_agent: &'a str,
    pub accept: &'a str,
    pub max_bytes: usize,
    pub timeout: Duration,
    pub error_prefix: &'a str,
}

pub struct Fetched {
    pub url: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

pub async fn fetch_public_url(url_str: &str, opts: &FetchOptions<'_>) -> Result<Fetched, String> {
    let prefix = opts.error_prefix;
    let mut url =
        reqwest::Url::parse(url_str).map_err(|e| format!("{prefix}: invalid URL: {e}"))?;
    check_url_shape(&url, prefix)?;

    let client = reqwest::Client::builder()
        .timeout(opts.timeout)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("{prefix}: HTTP client: {e}"))?;

    for _ in 0..=MAX_REDIRECTS {
        check_url_public(&url, prefix).await?;
        let resp = client
            .get(url.clone())
            .header(header::USER_AGENT, opts.user_agent)
            .header(header::ACCEPT, opts.accept)
            .send()
            .await
            .map_err(|e| format!("{prefix}: {e}"))?;

        let status = resp.status();
        if status.is_redirection() {
            let loc = resp
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| format!("{prefix}: redirect without Location"))?;
            url = url
                .join(loc)
                .map_err(|e| format!("{prefix}: bad redirect: {e}"))?;
            check_url_shape(&url, prefix)?;
            continue;
        }
        if !status.is_success() {
            return Err(format!("{prefix}: HTTP status {}", status.as_u16()));
        }

        let content_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if let Some(len) = resp.content_length()
            && len > opts.max_bytes as u64
        {
            return Err(format!("{prefix}: response is too large"));
        }

        let body = read_body_capped(resp, opts.max_bytes, prefix).await?;
        return Ok(Fetched {
            url: url.to_string(),
            content_type,
            body,
        });
    }
    Err(format!("{prefix}: too many redirects"))
}

async fn read_body_capped(
    resp: reqwest::Response,
    max_bytes: usize,
    prefix: &str,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("{prefix}: {e}"))?;
        let next_len = out.len().saturating_add(chunk.len());
        if next_len > max_bytes {
            return Err(format!("{prefix}: response is too large"));
        }
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

pub fn check_url_shape(url: &reqwest::Url, error_prefix: &str) -> Result<(), String> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!("{error_prefix}: only http(s) URLs are allowed"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!(
            "{error_prefix}: URLs with credentials are not allowed"
        ));
    }
    let Some(host) = url.host_str() else {
        return Err(format!("{error_prefix}: URL host is required"));
    };
    if hostname_is_blocked(host) {
        return Err(format!("{error_prefix}: host is not allowed"));
    }
    if let Some(ip) = parse_host_ip(host)
        && ip_is_disallowed(ip)
    {
        return Err(format!("{error_prefix}: host is not allowed"));
    }
    Ok(())
}

async fn check_url_public(url: &reqwest::Url, error_prefix: &str) -> Result<(), String> {
    check_url_shape(url, error_prefix)?;
    let Some(host) = url.host_str() else {
        return Err(format!("{error_prefix}: URL host is required"));
    };
    // Literal IPs were already checked in check_url_shape.
    if parse_host_ip(host).is_some() {
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(80);
    let lookup = format!("{host}:{port}");
    let addrs = lookup_host(&lookup)
        .await
        .map_err(|e| format!("{error_prefix}: DNS: {e}"))?;
    let mut any = false;
    for addr in addrs {
        any = true;
        if ip_is_disallowed(addr.ip()) {
            return Err(format!(
                "{error_prefix}: host resolves to a private address"
            ));
        }
    }
    if !any {
        return Err(format!("{error_prefix}: host did not resolve"));
    }
    Ok(())
}

fn parse_host_ip(host: &str) -> Option<IpAddr> {
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    host.parse().ok()
}

fn hostname_is_blocked(host: &str) -> bool {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();
    h == "localhost"
        || h.ends_with(".localhost")
        || h == "metadata.google.internal"
        || h.ends_with(".internal")
        || h.ends_with(".local")
        || h.ends_with(".lan")
        || h.ends_with(".home.arpa")
}

fn ip_is_disallowed(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.is_multicast()
                || is_cgnat(v4)
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return ip_is_disallowed(IpAddr::V4(v4));
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
        }
    }
}

fn is_cgnat(v4: std::net::Ipv4Addr) -> bool {
    let o = v4.octets();
    o.first().copied() == Some(100) && o.get(1).is_some_and(|b| (64..=127).contains(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_private_and_local_urls() {
        let blocked = [
            "file:///etc/passwd",
            "ftp://example.com/",
            "http://127.0.0.1/",
            "https://localhost/secret",
            "http://192.168.0.1/",
            "http://10.0.0.1/admin",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/",
            "http://user:pass@example.com/",
            "javascript:alert(1)",
            "http://foo.localhost/",
            "http://printer.local/",
        ];
        for u in blocked {
            match reqwest::Url::parse(u) {
                Ok(url) => assert!(check_url_shape(&url, "fetch").is_err(), "should reject {u}"),
                Err(_) => {}
            }
        }
    }

    #[test]
    fn accepts_public_https() {
        let url = reqwest::Url::parse("https://example.com/path?q=1").expect("parse");
        assert!(check_url_shape(&url, "fetch").is_ok());
    }
}
