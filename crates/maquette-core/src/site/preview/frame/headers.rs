//! What the frame proxy changes in the headers that pass through it, and
//! the dev server it may forward to.
//!
//! On the way in: the hop by hop headers go, `Host` names the dev server,
//! and an `Origin` or `Referer` of the proxy reads as the dev server's, so
//! dev tools that check where a request came from (server actions, HMR
//! sockets) answer as they would in a browser tab. On the way out: the hop
//! by hop headers go, `X-Frame-Options` goes, the `frame-ancestors`
//! directive leaves `Content-Security-Policy` and its `-Report-Only`
//! twin (every other directive stays as it was), and a `Location` on the
//! dev server points at the proxy instead.

use std::net::{IpAddr, Ipv6Addr};

use hyper::header::{
    CONNECTION, CONTENT_SECURITY_POLICY, CONTENT_SECURITY_POLICY_REPORT_ONLY, HOST, HeaderMap,
    HeaderName, HeaderValue, LOCATION, ORIGIN, REFERER, UPGRADE, X_FRAME_OPTIONS,
};

use crate::error::{CoreError, CoreResult};

/// Hop by hop headers: they describe one connection, so a proxy never
/// passes them on (`Connection` and `Upgrade` come back for an upgrade).
const HOP_BY_HOP: [&str; 9] = [
    "connection",
    "keep-alive",
    "proxy-connection",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// The loopback dev server a frame proxy forwards to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upstream {
    /// As written in its address: `localhost`, `127.0.0.1` or `[::1]`.
    host: String,
    port: u16,
}

impl Upstream {
    /// The dev server at `url`, which must be plain `http` on this machine.
    ///
    /// # Errors
    ///
    /// Refuses any other scheme and any host that is not loopback.
    pub fn parse(url: &str) -> CoreResult<Self> {
        let refuse = |why: &str| CoreError::Invalid(format!("Maquette frames {why}, not {url}."));
        let rest = url
            .strip_prefix("http://")
            .ok_or_else(|| refuse("only a plain http dev server"))?;
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if authority.contains('@') {
            return Err(refuse("a dev server address without a user"));
        }
        let (host, port) = split_authority(authority).ok_or_else(|| refuse("a host and port"))?;
        if !is_loopback(&host) {
            return Err(refuse("only a dev server on this machine"));
        }
        Ok(Self { host, port })
    }

    /// `localhost:3010`: what `Host` says.
    pub fn authority(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// The name to look up, without brackets.
    pub fn lookup_host(&self) -> &str {
        self.host.trim_start_matches('[').trim_end_matches(']')
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    /// `http://localhost:3010`.
    pub fn origin(&self) -> String {
        format!("http://{}", self.authority())
    }

    /// What follows the dev server's origin in `url`, from its `/`, when
    /// `url` is on the dev server under any of its loopback names.
    pub fn rest_of<'a>(&self, url: &'a str) -> Option<&'a str> {
        let rest = url.strip_prefix("http://")?;
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        let (host, port) = split_authority(authority)?;
        let same_host = host.eq_ignore_ascii_case(&self.host) || is_loopback(&host);
        (same_host && port == self.port).then_some(tail)
    }
}

/// `host:port` or `[v6]:port`, port 80 when there is none.
fn split_authority(authority: &str) -> Option<(String, u16)> {
    let (host, port) = if let Some(v6) = authority.strip_prefix('[') {
        let (inside, after) = v6.split_once(']')?;
        (format!("[{inside}]"), after.strip_prefix(':'))
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), Some(p)),
            None => (authority.to_string(), None),
        }
    };
    let port = match port {
        Some(p) => p.parse().ok()?,
        None => 80,
    };
    (!host.is_empty()).then(|| (host.to_ascii_lowercase(), port))
}

fn is_loopback(host: &str) -> bool {
    if host == "localhost" {
        return true;
    }
    if let Some(v6) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        return v6.parse::<Ipv6Addr>().is_ok_and(|ip| ip.is_loopback());
    }
    host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// True when the request asks to switch protocols (a WebSocket).
pub(super) fn wants_upgrade(headers: &HeaderMap) -> bool {
    headers.contains_key(UPGRADE) && listed(headers).any(|n| n.eq_ignore_ascii_case("upgrade"))
}

fn listed(headers: &HeaderMap) -> impl Iterator<Item = &str> {
    headers
        .get_all(CONNECTION)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|n| !n.is_empty())
}

/// Drops the hop by hop headers and the ones `Connection` names; an
/// upgrade keeps `Upgrade` and says `Connection: upgrade` again.
fn strip_hop_by_hop(headers: &mut HeaderMap, upgrade: bool) {
    let named: Vec<HeaderName> = listed(headers)
        .filter_map(|n| HeaderName::from_bytes(n.as_bytes()).ok())
        .filter(|n| !(upgrade && *n == UPGRADE))
        .collect();
    for name in named {
        headers.remove(name);
    }
    let kept = headers.get(UPGRADE).cloned();
    for name in HOP_BY_HOP {
        headers.remove(name);
    }
    if let (true, Some(protocol)) = (upgrade, kept) {
        headers.insert(UPGRADE, protocol);
        headers.insert(CONNECTION, HeaderValue::from_static("upgrade"));
    }
}

/// The request as the dev server should see it.
pub(super) fn fix_request(
    headers: &mut HeaderMap,
    upstream: &Upstream,
    proxy: &str,
    upgrade: bool,
) {
    strip_hop_by_hop(headers, upgrade);
    if let Ok(host) = HeaderValue::from_str(&upstream.authority()) {
        headers.insert(HOST, host);
    }
    for name in [ORIGIN, REFERER] {
        let moved = headers
            .get(&name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix(proxy))
            .filter(|tail| tail.is_empty() || tail.starts_with(['/', '?', '#']))
            .and_then(|tail| HeaderValue::from_str(&format!("{}{tail}", upstream.origin())).ok());
        if let Some(value) = moved {
            headers.insert(name, value);
        }
    }
}

/// The response as the frame may show it.
pub(super) fn fix_response(
    headers: &mut HeaderMap,
    upstream: &Upstream,
    proxy: &str,
    upgrade: bool,
) {
    strip_hop_by_hop(headers, upgrade);
    headers.remove(X_FRAME_OPTIONS);
    for name in [CONTENT_SECURITY_POLICY, CONTENT_SECURITY_POLICY_REPORT_ONLY] {
        strip_frame_ancestors(headers, &name);
    }
    let moved = headers
        .get(LOCATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| upstream.rest_of(v))
        .map(|tail| if tail.is_empty() { "/" } else { tail })
        .and_then(|tail| HeaderValue::from_str(&format!("{proxy}{tail}")).ok());
    if let Some(location) = moved {
        headers.insert(LOCATION, location);
    }
}

/// Every `name` header without its `frame-ancestors` directive. A header
/// without one is left exactly as it came, and one with nothing else goes.
fn strip_frame_ancestors(headers: &mut HeaderMap, name: &HeaderName) {
    let values: Vec<HeaderValue> = headers.get_all(name).iter().cloned().collect();
    let has = |v: &HeaderValue| {
        v.to_str()
            .is_ok_and(|s| without_frame_ancestors(s).is_some())
    };
    if !values.iter().any(has) {
        return;
    }
    headers.remove(name);
    for value in values {
        let next = match value.to_str().ok().and_then(without_frame_ancestors) {
            Some(Some(policy)) => HeaderValue::from_str(&policy).ok(),
            Some(None) => None,
            None => Some(value),
        };
        if let Some(next) = next {
            headers.append(name.clone(), next);
        }
    }
}

/// `None` when `policy` has no `frame-ancestors`; otherwise the policy
/// without it (`Some(None)` when nothing else is left). Policies split on
/// `,` and directives on `;`, as the CSP spec reads them.
pub(super) fn without_frame_ancestors(policy: &str) -> Option<Option<String>> {
    let is_it = |d: &str| {
        d.split_ascii_whitespace()
            .next()
            .is_some_and(|n| n.eq_ignore_ascii_case("frame-ancestors"))
    };
    if !policy.split([',', ';']).any(is_it) {
        return None;
    }
    let policies: Vec<String> = policy
        .split(',')
        .filter_map(|p| {
            let kept: Vec<&str> = p
                .split(';')
                .map(str::trim)
                .filter(|d| !d.is_empty() && !is_it(d))
                .collect();
            (!kept.is_empty()).then(|| kept.join("; "))
        })
        .collect();
    Some((!policies.is_empty()).then(|| policies.join(", ")))
}
