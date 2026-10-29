//! The page bridge shim: the script the frame proxy serves at
//! [`SHIM_PATH`] on its own origin, and the tag it adds to every page it
//! frames, so a site without the page bridge still follows the Page view's
//! scrubber. Served from the proxy's origin, the script passes a site CSP
//! that allows `script-src 'self'`; nothing inline is added and no other
//! directive changes.
//!
//! Only a page load is touched: a `GET` for a document or a frame that
//! answers `text/html`. Its request goes without `Accept-Encoding`, so the
//! page comes back plain; one that still comes gzip or deflate is decoded
//! as it streams. The tag goes right after `<head>`, else before
//! `</head>`, else after `<body>`; a page that shows none of them in its
//! first [`LOOK_AHEAD`] bytes passes through as it came. The page keeps
//! streaming: only the bytes up to that spot are held back, and the length
//! it no longer has goes, so it is sent chunked. Every other body passes
//! through untouched.

use std::io::{self, Write as _};
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use flate2::write::{GzDecoder, ZlibDecoder};
use hyper::body::{Body as HttpBody, Bytes, Frame, Incoming, SizeHint};
use hyper::header::{
    ACCEPT, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, HeaderMap,
};
use hyper::{Method, StatusCode};

/// Where the shim is served; the proxy never forwards anything under `/__maquette/`.
pub(super) const SHIM_PATH: &str = "/__maquette/bridge.js";
pub(super) const RESERVED: &str = "/__maquette/";
pub(super) const SHIM: &str = include_str!("shim.js");

/// What goes into a page.
pub(super) const TAG: &str = "<script src=\"/__maquette/bridge.js\" defer></script>";

/// How much of a page is held back looking for its `<head>`.
const LOOK_AHEAD: usize = 256 * 1024;

pub(super) type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// True for a request that loads a page: a `GET` for a document or a
/// frame, or, from a browser that does not say, one that accepts HTML.
pub(super) fn wants_page(method: &Method, headers: &HeaderMap) -> bool {
    if method != Method::GET {
        return false;
    }
    let text = |name| {
        headers
            .get(name)
            .and_then(|v: &hyper::header::HeaderValue| v.to_str().ok())
    };
    match text("sec-fetch-dest") {
        Some(dest) => ["document", "iframe", "frame"]
            .iter()
            .any(|d| dest.eq_ignore_ascii_case(d)),
        None => text(ACCEPT.as_str()).is_some_and(|a| a.contains("text/html")),
    }
}

/// A page request asks for the page plain, so the tag can go in.
pub(super) fn fix_page_request(headers: &mut HeaderMap) {
    headers.remove(ACCEPT_ENCODING);
}

/// How to read the answer to a page request, or `None` to pass it through:
/// it is not HTML, has no body, or comes in an encoding the proxy cannot
/// read (brotli, say).
pub(super) fn page_body(status: StatusCode, headers: &HeaderMap) -> Option<Decode> {
    if status.is_informational()
        || status == StatusCode::NO_CONTENT
        || status == StatusCode::NOT_MODIFIED
    {
        return None;
    }
    let html = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("text/html"));
    if !html {
        return None;
    }
    let encodings: Vec<String> = headers
        .get_all(CONTENT_ENCODING)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(|e| e.trim().to_ascii_lowercase())
        .filter(|e| !e.is_empty() && e != "identity")
        .collect();
    match encodings.as_slice() {
        [] => Some(Decode::Plain),
        [one] if one == "gzip" || one == "x-gzip" => Some(Decode::Gzip(GzDecoder::new(Vec::new()))),
        [one] if one == "deflate" => Some(Decode::Zlib(ZlibDecoder::new(Vec::new()))),
        _ => None,
    }
}

/// The page's headers once its body changes: decoded and of a new length.
pub(super) fn fix_page_response(headers: &mut HeaderMap) {
    headers.remove(CONTENT_LENGTH);
    headers.remove(CONTENT_ENCODING);
}

/// Where the tag goes in the start of a page, or `None` while it cannot
/// be told yet.
pub(super) fn spot(html: &[u8]) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    if let Some(open) = find_tag(&lower, b"<head") {
        return tag_end(&lower, open);
    }
    if let Some(close) = find_tag(&lower, b"</head") {
        return Some(close);
    }
    find_tag(&lower, b"<body").and_then(|open| tag_end(&lower, open))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The first `name` that is that tag, not a longer one (`<header>`).
fn find_tag(html: &[u8], name: &[u8]) -> Option<usize> {
    let mut from = 0;
    while let Some(at) = find(&html[from..], name).map(|i| from + i) {
        match html.get(at + name.len()) {
            Some(b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r' | b'\x0c') => return Some(at),
            Some(_) => from = at + name.len(),
            None => return None,
        }
    }
    None
}

fn tag_end(html: &[u8], open: usize) -> Option<usize> {
    html[open..]
        .iter()
        .position(|&b| b == b'>')
        .map(|end| open + end + 1)
}

/// How the page's body is encoded.
pub(super) enum Decode {
    Plain,
    Gzip(GzDecoder<Vec<u8>>),
    Zlib(ZlibDecoder<Vec<u8>>),
}

impl Decode {
    fn feed(&mut self, data: Bytes) -> io::Result<Bytes> {
        match self {
            Self::Plain => Ok(data),
            Self::Gzip(d) => {
                d.write_all(&data)?;
                Ok(Bytes::from(std::mem::take(d.get_mut())))
            }
            Self::Zlib(d) => {
                d.write_all(&data)?;
                Ok(Bytes::from(std::mem::take(d.get_mut())))
            }
        }
    }

    fn finish(&mut self) -> io::Result<Bytes> {
        let rest = match self {
            Self::Plain => return Ok(Bytes::new()),
            Self::Gzip(d) => {
                d.try_finish()?;
                std::mem::take(d.get_mut())
            }
            Self::Zlib(d) => {
                d.try_finish()?;
                std::mem::take(d.get_mut())
            }
        };
        Ok(Bytes::from(rest))
    }
}

/// A page's body with the tag in it, streamed: held back only until the
/// spot for the tag shows up.
pub(super) struct Injected {
    inner: Incoming,
    decode: Decode,
    /// What came before the spot; `None` once the tag is in or given up.
    held: Option<Vec<u8>>,
    done: bool,
}

impl Injected {
    pub(super) fn new(inner: Incoming, decode: Decode) -> Self {
        Self {
            inner,
            decode,
            held: Some(Vec::new()),
            done: false,
        }
    }

    /// What can go out now of `plain`, the next decoded bytes.
    fn pass(&mut self, plain: Bytes, end: bool) -> Bytes {
        let Some(held) = self.held.as_mut() else {
            return plain;
        };
        held.extend_from_slice(&plain);
        if let Some(at) = spot(held) {
            let mut out = Vec::with_capacity(held.len() + TAG.len());
            out.extend_from_slice(&held[..at]);
            out.extend_from_slice(TAG.as_bytes());
            out.extend_from_slice(&held[at..]);
            self.held = None;
            return Bytes::from(out);
        }
        if end || held.len() > LOOK_AHEAD {
            return Bytes::from(self.held.take().unwrap_or_default());
        }
        Bytes::new()
    }
}

impl HttpBody for Injected {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, BoxError>>> {
        let this = &mut *self;
        while !this.done {
            let out = match ready!(Pin::new(&mut this.inner).poll_frame(cx)) {
                // Trailers have no place once the body changed.
                Some(Ok(frame)) => match frame.into_data() {
                    Ok(data) => {
                        let plain = this.decode.feed(data)?;
                        this.pass(plain, false)
                    }
                    Err(_) => continue,
                },
                Some(Err(err)) => return Poll::Ready(Some(Err(err.into()))),
                None => {
                    this.done = true;
                    let rest = this.decode.finish()?;
                    this.pass(rest, true)
                }
            };
            if !out.is_empty() {
                return Poll::Ready(Some(Ok(Frame::data(out))));
            }
        }
        Poll::Ready(None)
    }

    fn is_end_stream(&self) -> bool {
        self.done
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::default()
    }
}
