//! The outbound SSRF guard (target-system-design §12.5 `net_guard`; T18; ADR-0008 §8).
//!
//! This module is the only product code that builds an outbound HTTP client (`cargo xtask
//! check`, rule `egress`). A user-supplied URL is usable only as a [`GuardedUrl`], which only
//! [`check`] builds: `https` on 443 (or, with [`OutboundPolicy::allow_http`], `http` on 80),
//! and every address its host resolves to outside the forbidden classes. [`fetch`] connects
//! to the address `check` pinned, whatever DNS says later, and never follows a redirect.
//! URLs are checked at config write and again at call time, which defeats DNS rebinding.

use crate::BoxFuture;
use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

/// What outbound calls may use. `allow_http` is off by default (§12.5 "Ports").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OutboundPolicy {
    pub allow_http: bool,
}

/// Name resolution, injected so tests can exercise private answers and rebinding.
pub trait Resolver: Send + Sync {
    fn resolve<'a>(&'a self, host: &'a str) -> BoxFuture<'a, std::io::Result<Vec<IpAddr>>>;
}

/// The operating system's resolver.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemResolver;

impl Resolver for SystemResolver {
    fn resolve<'a>(&'a self, host: &'a str) -> BoxFuture<'a, std::io::Result<Vec<IpAddr>>> {
        Box::pin(async move {
            Ok(tokio::net::lookup_host((host, 0))
                .await?
                .map(|a| a.ip())
                .collect())
        })
    }
}

/// A URL that passed [`check`], pinned to one address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedUrl {
    url: String,
    host: String,
    pinned: SocketAddr,
}

impl GuardedUrl {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn pinned(&self) -> SocketAddr {
        self.pinned
    }

    /// A URL pinned to `addr` without the check, so tests can aim the client at a loopback
    /// server. Tests only.
    #[cfg(feature = "fault-injection")]
    pub fn pinned_for_test(url: &str, addr: SocketAddr) -> GuardedUrl {
        let host = Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_default();
        GuardedUrl {
            url: url.to_string(),
            host,
            pinned: addr,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NetGuardError {
    #[error("not a valid absolute URL")]
    InvalidUrl,
    #[error("scheme not allowed")]
    Scheme,
    #[error("the host does not resolve")]
    Unresolvable,
    #[error("{addr} is a {class} address")]
    Forbidden { addr: IpAddr, class: &'static str },
    #[error("port not allowed")]
    Port,
    #[error("redirect to {location} refused")]
    Redirect { location: String },
    #[error("http: {0}")]
    Http(String),
}

fn v4_class(ip: Ipv4Addr) -> Option<&'static str> {
    let [a, b, c, _] = ip.octets();
    match (a, b, c) {
        _ if ip.is_broadcast() => Some("broadcast"),
        (127, _, _) => Some("loopback"),
        (0, _, _) => Some("unspecified"),
        (10, _, _) => Some("private"),
        (172, 16..=31, _) => Some("private"),
        (192, 168, _) => Some("private"),
        (100, 64..=127, _) => Some("cgnat"),
        (169, 254, _) => Some("link_local"),
        (224..=239, _, _) => Some("multicast"),
        (192, 0, 2) | (198, 51, 100) | (203, 0, 113) => Some("documentation"),
        _ => None,
    }
}

fn v6_class(ip: Ipv6Addr) -> Option<&'static str> {
    let s = ip.segments();
    if ip.is_unspecified() {
        return Some("unspecified");
    }
    if ip.is_loopback() {
        return Some("loopback");
    }
    // IPv4-mapped (::ffff:a.b.c.d) and IPv4-compatible (::a.b.c.d) take the class of the
    // embedded address (§12.5).
    if s[..5] == [0; 5] && (s[5] == 0xffff || s[5] == 0) {
        let o = ip.octets();
        return v4_class(Ipv4Addr::new(o[12], o[13], o[14], o[15]));
    }
    match s[0] {
        x if x & 0xffc0 == 0xfe80 => Some("link_local"),
        x if x & 0xfe00 == 0xfc00 => Some("ula"),
        x if x & 0xff00 == 0xff00 => Some("multicast"),
        0x2001 if s[1] == 0x0db8 => Some("documentation"),
        _ => None,
    }
}

/// The forbidden class of `ip`, or `None` if it may be reached (ADR-0008 §8 "Classes").
pub fn classify(ip: IpAddr) -> Option<&'static str> {
    match ip {
        IpAddr::V4(v4) => v4_class(v4),
        IpAddr::V6(v6) => v6_class(v6),
    }
}

fn refuse(addr: IpAddr) -> Result<(), NetGuardError> {
    match classify(addr) {
        Some(class) => Err(NetGuardError::Forbidden { addr, class }),
        None => Ok(()),
    }
}

/// Checks `url` under `policy`, resolving names through `resolver` (ADR-0008 §8 `check`).
pub async fn check(
    url: &str,
    policy: &OutboundPolicy,
    resolver: &dyn Resolver,
) -> Result<GuardedUrl, NetGuardError> {
    let parsed = Url::parse(url).map_err(|_| NetGuardError::InvalidUrl)?;
    let port = match parsed.scheme() {
        "https" => 443,
        "http" if policy.allow_http => 80,
        // http without the opt-in is refused as a port, as ADR-0008 §8 specifies.
        "http" => return Err(NetGuardError::Port),
        _ => return Err(NetGuardError::Scheme),
    };
    if parsed.port_or_known_default() != Some(port) {
        return Err(NetGuardError::Port);
    }
    let host = parsed
        .host_str()
        .filter(|h| !h.is_empty())
        .ok_or(NetGuardError::InvalidUrl)?
        .to_string();
    let literal = match host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        Some(v6) => Some(IpAddr::V6(
            v6.parse().map_err(|_| NetGuardError::InvalidUrl)?,
        )),
        None => host.parse::<Ipv4Addr>().ok().map(IpAddr::V4),
    };
    let first = match literal {
        Some(ip) => {
            refuse(ip)?;
            ip
        }
        None => {
            let answer = resolver
                .resolve(&host)
                .await
                .map_err(|_| NetGuardError::Unresolvable)?;
            // ADR-0008 ambiguity 10: any forbidden address in the answer refuses the URL.
            for ip in &answer {
                refuse(*ip)?;
            }
            *answer.first().ok_or(NetGuardError::Unresolvable)?
        }
    };
    Ok(GuardedUrl {
        url: url.to_string(),
        host,
        pinned: SocketAddr::new(first, port),
    })
}

/// The result of [`fetch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub status: u16,
    pub body: Vec<u8>,
}

fn http(e: reqwest::Error) -> NetGuardError {
    NetGuardError::Http(e.to_string())
}

/// GET `g` from its pinned address, with `Host` from the URL; any 3xx is
/// [`NetGuardError::Redirect`]. A caller that follows re-runs [`check`] on the location.
pub async fn fetch(g: &GuardedUrl, timeout: Duration) -> Result<Fetched, NetGuardError> {
    let url = Url::parse(&g.url).map_err(|_| NetGuardError::InvalidUrl)?;
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(timeout);
    let domain = url.host_str().filter(|h| !h.starts_with('['));
    if let Some(d) = domain.filter(|d| d.parse::<Ipv4Addr>().is_err()) {
        // The pin: this name connects to the checked address only.
        builder = builder.resolve(d, g.pinned);
    }
    let client = builder.build().map_err(http)?;
    let resp = client.get(url).send().await.map_err(http)?;
    let status = resp.status();
    if status.is_redirection() {
        let location = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        return Err(NetGuardError::Redirect { location });
    }
    let body = resp.bytes().await.map_err(http)?.to_vec();
    Ok(Fetched {
        status: status.as_u16(),
        body,
    })
}

/// A client for platform-fixed hosts (LLM APIs; §12.5 "fixed_host"): https only, no
/// redirects, no proxy from the environment.
pub fn fixed_host_client(timeout: Duration) -> Result<reqwest::Client, NetGuardError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .no_proxy()
        .timeout(timeout)
        .build()
        .map_err(http)
}
