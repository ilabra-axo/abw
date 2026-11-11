//! T18: no user-supplied URL reaches a private address (target-system-design §12.5, §15.1
//! `ssrf_guard`; capabilities.md §14 "Guard outbound calls"; ADR-0008 §8).
//!
//! DNS is injected through `Resolver`, so names that resolve to private addresses, mixed
//! answers and rebinding are exercised without a network. The redirect and pinning tests
//! talk to a throwaway HTTP server on loopback through `GuardedUrl::pinned_for_test`, the
//! only way to aim the client at an address `check` would refuse.

use abw_agent::BoxFuture;
use abw_agent::egress::{self, GuardedUrl, NetGuardError, OutboundPolicy, Resolver};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener};
use std::sync::Mutex;
use std::time::Duration;

/// A resolver with a fixed answer per host; a host with a list of answers returns the next
/// one on each call (the last one repeats), which simulates DNS rebinding.
struct StubResolver {
    answers: Mutex<HashMap<String, Vec<Vec<IpAddr>>>>,
}

impl StubResolver {
    fn new(entries: &[(&str, &[&[&str]])]) -> StubResolver {
        let mut map = HashMap::new();
        for (host, seq) in entries {
            let parsed: Vec<Vec<IpAddr>> = seq
                .iter()
                .map(|ips| ips.iter().map(|s| ip(s)).collect())
                .collect();
            map.insert(host.to_string(), parsed);
        }
        StubResolver {
            answers: Mutex::new(map),
        }
    }
}

impl Resolver for StubResolver {
    fn resolve<'a>(&'a self, host: &'a str) -> BoxFuture<'a, std::io::Result<Vec<IpAddr>>> {
        Box::pin(async move {
            let mut map = match self.answers.lock() {
                Ok(m) => m,
                Err(e) => panic!("{e}"),
            };
            match map.get_mut(host) {
                Some(seq) if seq.len() > 1 => Ok(seq.remove(0)),
                Some(seq) => Ok(seq.first().cloned().unwrap_or_default()),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no such host {host}"),
                )),
            }
        })
    }
}

fn ip(s: &str) -> IpAddr {
    match s.parse() {
        Ok(a) => a,
        Err(e) => panic!("{s}: {e}"),
    }
}

fn https_only() -> OutboundPolicy {
    OutboundPolicy { allow_http: false }
}

fn no_dns() -> StubResolver {
    StubResolver::new(&[])
}

async fn refused(url: &str, resolver: &StubResolver) -> NetGuardError {
    match egress::check(url, &https_only(), resolver).await {
        Err(e) => e,
        Ok(g) => panic!("{url} was allowed, pinned to {}", g.pinned()),
    }
}

// @req T18, CAP-14.05
#[test]
fn private_and_special_ipv4_ranges_are_classified_as_forbidden() {
    for (addr, class) in [
        ("127.0.0.1", "loopback"),
        ("127.255.255.254", "loopback"),
        ("0.0.0.0", "unspecified"),
        ("10.0.0.1", "private"),
        ("10.255.255.255", "private"),
        ("172.16.0.1", "private"),
        ("172.31.255.255", "private"),
        ("192.168.1.1", "private"),
        ("100.64.0.1", "cgnat"),
        ("100.127.255.255", "cgnat"),
        ("169.254.0.1", "link_local"),
        ("169.254.169.254", "link_local"),
        ("224.0.0.1", "multicast"),
        ("239.255.255.250", "multicast"),
        ("255.255.255.255", "broadcast"),
        ("192.0.2.10", "documentation"),
        ("198.51.100.7", "documentation"),
        ("203.0.113.99", "documentation"),
    ] {
        assert_eq!(egress::classify(ip(addr)), Some(class), "{addr}");
    }
}

// @req T18, CAP-14.05
#[test]
fn private_and_special_ipv6_ranges_and_embedded_ipv4_are_forbidden() {
    for (addr, class) in [
        ("::1", "loopback"),
        ("::", "unspecified"),
        ("fe80::1", "link_local"),
        ("febf::ffff", "link_local"),
        ("fc00::1", "ula"),
        ("fd12:3456:789a::1", "ula"),
        ("ff02::1", "multicast"),
        ("2001:db8::1", "documentation"),
        // IPv4-mapped and IPv4-compatible forms take the embedded address's class.
        ("::ffff:127.0.0.1", "loopback"),
        ("::ffff:10.1.2.3", "private"),
        ("::ffff:169.254.169.254", "link_local"),
        ("::ffff:192.168.0.1", "private"),
        ("::127.0.0.1", "loopback"),
        ("::10.0.0.1", "private"),
    ] {
        assert_eq!(egress::classify(ip(addr)), Some(class), "{addr}");
    }
}

// @req T18, CAP-14.05
#[test]
fn public_addresses_are_not_forbidden() {
    for addr in [
        "1.1.1.1",
        "8.8.8.8",
        "93.184.216.34",
        "172.32.0.1",
        "100.128.0.1",
        "169.255.0.1",
        "11.0.0.1",
        "2606:4700:4700::1111",
        "2a00:1450:4001:80b::200e",
        "::ffff:93.184.216.34",
    ] {
        assert_eq!(egress::classify(ip(addr)), None, "{addr}");
    }
}

// @req T18, CAP-14.05
#[tokio::test]
async fn ip_literal_urls_to_private_addresses_are_refused() {
    let dns = no_dns();
    for (url, addr, class) in [
        ("https://127.0.0.1/", "127.0.0.1", "loopback"),
        ("https://10.0.0.5/internal", "10.0.0.5", "private"),
        ("https://172.20.1.1/", "172.20.1.1", "private"),
        ("https://192.168.0.10/admin", "192.168.0.10", "private"),
        (
            "https://169.254.169.254/latest/meta-data/",
            "169.254.169.254",
            "link_local",
        ),
        ("https://[::1]/", "::1", "loopback"),
        ("https://[fd00::1]/", "fd00::1", "ula"),
        ("https://[fe80::1]/", "fe80::1", "link_local"),
        (
            "https://[::ffff:127.0.0.1]/",
            "::ffff:127.0.0.1",
            "loopback",
        ),
        (
            "https://[::ffff:a9fe:a9fe]/",
            "::ffff:169.254.169.254",
            "link_local",
        ),
    ] {
        match refused(url, &dns).await {
            NetGuardError::Forbidden {
                addr: got,
                class: c,
            } => {
                assert_eq!((got, c), (ip(addr), class), "{url}");
            }
            other => panic!("{url}: expected Forbidden, got {other:?}"),
        }
    }
}

// @req T18, CAP-14.05
#[tokio::test]
async fn a_name_resolving_to_a_private_address_is_refused() {
    let dns = StubResolver::new(&[
        ("metadata.attacker.example", &[&["169.254.169.254"]]),
        ("internal.attacker.example", &[&["10.1.2.3"]]),
        ("v6.attacker.example", &[&["::1"]]),
        ("mapped.attacker.example", &[&["::ffff:192.168.1.1"]]),
        // One bad address in the answer is enough to refuse.
        ("mixed.attacker.example", &[&["93.184.216.34", "127.0.0.1"]]),
    ]);
    for (host, class) in [
        ("metadata.attacker.example", "link_local"),
        ("internal.attacker.example", "private"),
        ("v6.attacker.example", "loopback"),
        ("mapped.attacker.example", "private"),
        ("mixed.attacker.example", "loopback"),
    ] {
        match refused(&format!("https://{host}/x"), &dns).await {
            NetGuardError::Forbidden { class: c, .. } => assert_eq!(c, class, "{host}"),
            other => panic!("{host}: expected Forbidden, got {other:?}"),
        }
    }
}

// @req T18, CAP-14.05
#[tokio::test]
async fn rebinding_is_caught_by_the_check_at_call_time() {
    let dns = StubResolver::new(&[(
        "rebind.attacker.example",
        &[&["93.184.216.34"], &["127.0.0.1"]],
    )]);
    let url = "https://rebind.attacker.example/hook";
    // At config write the name looks public...
    match egress::check(url, &https_only(), &dns).await {
        Ok(g) => assert_eq!(g.pinned().ip(), ip("93.184.216.34")),
        Err(e) => panic!("config-time check: {e:?}"),
    }
    // ...and at call time it points at loopback, so the second check refuses it.
    match refused(url, &dns).await {
        NetGuardError::Forbidden {
            class: "loopback", ..
        } => {}
        other => panic!("rebinding not caught: {other:?}"),
    }
}

// @req T18, CAP-14.05
#[tokio::test]
async fn a_public_address_is_allowed_and_pinned() {
    let dns = StubResolver::new(&[
        ("api.example.com", &[&["93.184.216.34", "93.184.216.35"]]),
        ("v6.example.com", &[&["2606:4700:4700::1111"]]),
    ]);
    let g = match egress::check("https://api.example.com/v1/data?q=1", &https_only(), &dns).await {
        Ok(g) => g,
        Err(e) => panic!("a public URL was refused: {e:?}"),
    };
    assert_eq!(
        g.pinned(),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)), 443)
    );
    assert_eq!(g.host(), "api.example.com");
    assert_eq!(g.url(), "https://api.example.com/v1/data?q=1");
    let g6 = match egress::check("https://v6.example.com/", &https_only(), &dns).await {
        Ok(g) => g,
        Err(e) => panic!("{e:?}"),
    };
    assert_eq!(
        g6.pinned(),
        SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111)),
            443
        )
    );
    match egress::check("https://1.1.1.1/dns-query", &https_only(), &no_dns()).await {
        Ok(g) => assert_eq!(g.pinned(), SocketAddr::new(ip("1.1.1.1"), 443)),
        Err(e) => panic!("{e:?}"),
    }
}

// @req T18, CAP-14.05
#[tokio::test]
async fn only_https_on_443_and_opt_in_http_on_80_are_allowed() {
    let dns = StubResolver::new(&[("api.example.com", &[&["93.184.216.34"]])]);
    for (url, want) in [
        ("ftp://api.example.com/file", NetGuardError::Scheme),
        ("file:///etc/passwd", NetGuardError::Scheme),
        ("gopher://api.example.com/", NetGuardError::Scheme),
        ("https://api.example.com:8443/", NetGuardError::Port),
        ("https://api.example.com:22/", NetGuardError::Port),
        ("http://api.example.com/", NetGuardError::Port),
        ("not a url", NetGuardError::InvalidUrl),
    ] {
        assert_eq!(refused(url, &dns).await, want, "{url}");
    }
    let with_http = OutboundPolicy { allow_http: true };
    match egress::check("http://api.example.com/", &with_http, &dns).await {
        Ok(g) => assert_eq!(g.pinned().port(), 80),
        Err(e) => panic!("http with allow_http: {e:?}"),
    }
    match egress::check("http://api.example.com:8080/", &with_http, &dns).await {
        Err(NetGuardError::Port) => {}
        other => panic!("http on 8080: {other:?}"),
    }
    // Opting into http doesn't open private addresses.
    match egress::check("http://127.0.0.1/", &with_http, &dns).await {
        Err(NetGuardError::Forbidden { .. }) => {}
        other => panic!("http to loopback: {other:?}"),
    }
    assert!(!OutboundPolicy::default().allow_http);
}

// @req T18, CAP-14.05
#[tokio::test]
async fn an_unresolvable_name_is_refused() {
    let dns = StubResolver::new(&[("empty.example.com", &[&[]])]);
    assert_eq!(
        refused("https://empty.example.com/", &dns).await,
        NetGuardError::Unresolvable
    );
    assert_eq!(
        refused("https://missing.example.com/", &dns).await,
        NetGuardError::Unresolvable
    );
}

/// A one-shot HTTP server on loopback that answers the first request with `response` and
/// hands back the request it received.
fn one_shot_server(response: &'static str) -> (SocketAddr, std::thread::JoinHandle<String>) {
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => panic!("bind: {e}"),
    };
    let addr = match listener.local_addr() {
        Ok(a) => a,
        Err(e) => panic!("{e}"),
    };
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = match listener.accept() {
            Ok(s) => s,
            Err(e) => panic!("accept: {e}"),
        };
        let mut buf = [0u8; 4096];
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(e) => panic!("read: {e}"),
        };
        if let Err(e) = stream.write_all(response.as_bytes()) {
            panic!("write: {e}");
        }
        String::from_utf8_lossy(&buf[..n]).into_owned()
    });
    (addr, handle)
}

// @req T18, CAP-14.05
#[tokio::test]
async fn a_redirect_is_refused_and_following_it_rechecks_the_target() {
    let (addr, server) = one_shot_server(
        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/admin\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    let g = GuardedUrl::pinned_for_test(
        &format!("http://public.example:{}/start", addr.port()),
        addr,
    );
    match egress::fetch(&g, Duration::from_secs(5)).await {
        Err(NetGuardError::Redirect { location }) => {
            assert_eq!(location, "http://127.0.0.1:9/admin");
            // A caller that wants to follow re-checks the hop, and loopback is refused.
            match egress::check(&location, &OutboundPolicy { allow_http: true }, &no_dns()).await {
                Err(NetGuardError::Forbidden {
                    class: "loopback", ..
                })
                | Err(NetGuardError::Port) => {}
                other => panic!("the redirect target passed the guard: {other:?}"),
            }
        }
        other => panic!("a redirect was followed or misreported: {other:?}"),
    }
    let request = match server.join() {
        Ok(r) => r,
        Err(_) => panic!("server thread panicked"),
    };
    assert!(request.starts_with("GET /start "), "{request}");
}

// @req T18, CAP-14.05
#[tokio::test]
async fn the_client_connects_to_the_pinned_address_whatever_dns_says() {
    let (addr, server) = one_shot_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nhello",
    );
    // `pinned.invalid` never resolves; the request can only arrive through the pin.
    let g =
        GuardedUrl::pinned_for_test(&format!("http://pinned.invalid:{}/data", addr.port()), addr);
    match egress::fetch(&g, Duration::from_secs(5)).await {
        Ok(f) => {
            assert_eq!(f.status, 200);
            assert_eq!(f.body, b"hello");
        }
        Err(e) => panic!("fetch through the pin failed: {e:?}"),
    }
    let request = match server.join() {
        Ok(r) => r,
        Err(_) => panic!("server thread panicked"),
    };
    let host_header = request
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("host:"))
        .map(str::to_string);
    assert_eq!(
        host_header.map(|h| h.to_ascii_lowercase()),
        Some(format!("host: pinned.invalid:{}", addr.port())),
        "the Host header names the URL's host, not the pinned IP"
    );
}
