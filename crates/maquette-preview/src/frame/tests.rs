use std::io::{Read as _, Write as _};
use std::net::TcpStream as StdStream;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::Instant;

use http_body_util::{BodyExt as _, combinators::BoxBody};
use hyper::body::{Body as HttpBody, Frame};
use hyper::header::{HOST, LOCATION, ORIGIN};
use tokio::sync::mpsc;

use super::headers::without_frame_ancestors;
use super::inject::{SHIM, TAG, spot};
use super::*;

type UpBody = BoxBody<Bytes, Infallible>;

/// A body that sends what the channel gives it, as it comes.
struct Chunks(mpsc::UnboundedReceiver<Bytes>);

impl HttpBody for Chunks {
    type Data = Bytes;
    type Error = Infallible;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        self.0.poll_recv(cx).map(|b| b.map(|b| Ok(Frame::data(b))))
    }
}

fn full(text: &str) -> UpBody {
    Full::new(Bytes::from(text.to_string())).boxed()
}

/// A fake dev server that forbids framing, as most production sites do.
struct Fake {
    runtime: tokio::runtime::Runtime,
    port: u16,
    /// Releases the second half of `/stream`.
    release: mpsc::UnboundedSender<Bytes>,
}

#[derive(Default)]
struct Seen {
    host: Option<String>,
    origin: Option<String>,
    path: Option<String>,
    accept_encoding: Option<String>,
}

const PAGE: &str = "<!doctype html><html lang=\"en\"><head><title>A site</title></head>\
<body><header>Nav</header><p>the page</p></body></html>";

fn gzip(text: &str) -> Bytes {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(text.as_bytes()).expect("gzip");
    Bytes::from(encoder.finish().expect("gzip"))
}

fn fake(seen: Arc<Mutex<Seen>>) -> Fake {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let listener = {
        let _entered = runtime.enter();
        let std = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
        std.set_nonblocking(true).expect("nonblocking");
        tokio::net::TcpListener::from_std(std).expect("listener")
    };
    let port = listener.local_addr().expect("addr").port();
    let (release, held) = mpsc::unbounded_channel::<Bytes>();
    let held = Arc::new(tokio::sync::Mutex::new(Some(held)));
    runtime.spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let seen = Arc::clone(&seen);
            let held = Arc::clone(&held);
            tokio::spawn(async move {
                let service = hyper::service::service_fn(move |req| {
                    let seen = Arc::clone(&seen);
                    let held = Arc::clone(&held);
                    async move { Ok::<_, Infallible>(answer(req, port, &seen, &held).await) }
                });
                let _ = hyper::server::conn::http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .with_upgrades()
                    .await;
            });
        }
    });
    Fake {
        runtime,
        port,
        release,
    }
}

type Held = tokio::sync::Mutex<Option<mpsc::UnboundedReceiver<Bytes>>>;

async fn answer(
    mut req: Request<Incoming>,
    port: u16,
    seen: &Mutex<Seen>,
    held: &Held,
) -> Response<UpBody> {
    let header = |name| {
        req.headers()
            .get(name)
            .and_then(|v: &HeaderValue| v.to_str().ok())
            .map(str::to_string)
    };
    *seen.lock().expect("seen") = Seen {
        host: header(HOST),
        origin: header(ORIGIN),
        path: Some(req.uri().path().to_string()),
        accept_encoding: header(hyper::header::ACCEPT_ENCODING),
    };
    let path = req.uri().path().to_string();
    let mut res = match path.as_str() {
        "/stream" | "/stream-page" => {
            let (tx, rx) = mpsc::unbounded_channel();
            let first: &'static [u8] = if path == "/stream" {
                b"first-half "
            } else {
                b"<!doctype html><html><head><title>first-half</title>"
            };
            let _ = tx.send(Bytes::from_static(first));
            if let Some(mut later) = held.lock().await.take() {
                tokio::spawn(async move {
                    while let Some(chunk) = later.recv().await {
                        let _ = tx.send(chunk);
                    }
                });
            }
            let mut res = Response::new(Chunks(rx).boxed());
            if path == "/stream-page" {
                res.headers_mut()
                    .insert("content-type", HeaderValue::from_static("text/html"));
            }
            res
        }
        "/page" | "/page.gz" | "/app.js" => {
            let (body, kind) = match path.as_str() {
                "/app.js" => (full("<head>not a page</head>"), "text/javascript"),
                "/page.gz" => (Full::new(gzip(PAGE)).boxed(), "text/html; charset=utf-8"),
                _ => (full(PAGE), "text/html; charset=utf-8"),
            };
            let mut res = Response::new(body);
            res.headers_mut()
                .insert("content-type", HeaderValue::from_static(kind));
            if path == "/page.gz" {
                // Sent gzip whatever the request asked for.
                res.headers_mut()
                    .insert("content-encoding", HeaderValue::from_static("gzip"));
            }
            res
        }
        "/redirect" => {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::TEMPORARY_REDIRECT;
            let to = format!("http://localhost:{port}/next?a=1");
            res.headers_mut()
                .insert(LOCATION, HeaderValue::from_str(&to).expect("location"));
            res
        }
        "/away" => {
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::FOUND;
            res.headers_mut()
                .insert(LOCATION, HeaderValue::from_static("https://example.com/"));
            res
        }
        "/hmr" => {
            let upgrade = hyper::upgrade::on(&mut req);
            tokio::spawn(async move {
                let Ok(io) = upgrade.await else { return };
                let mut io = TokioIo::new(io);
                let (mut read, mut write) = tokio::io::split(&mut io);
                let _ = tokio::io::copy(&mut read, &mut write).await;
            });
            let mut res = Response::new(full(""));
            *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
            let h = res.headers_mut();
            h.insert("upgrade", HeaderValue::from_static("websocket"));
            h.insert("connection", HeaderValue::from_static("Upgrade"));
            return res;
        }
        _ => Response::new(full("<p>the site</p>")),
    };
    let h = res.headers_mut();
    h.insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'self'; frame-ancestors 'none'; img-src * data:"),
    );
    h.insert(
        "content-security-policy-report-only",
        HeaderValue::from_static("frame-ancestors 'self'"),
    );
    h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    h.insert("x-powered-by", HeaderValue::from_static("a dev server"));
    res
}

/// Sends a raw request and reads until `until` returns true or it closes.
fn exchange(port: u16, request: &str, until: impl Fn(&str) -> bool) -> (StdStream, String) {
    let mut stream = StdStream::connect((Ipv4Addr::LOCALHOST, port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    stream.write_all(request.as_bytes()).expect("write");
    let got = read_until(&mut stream, until);
    (stream, got)
}

fn read_until(stream: &mut StdStream, until: impl Fn(&str) -> bool) -> String {
    let mut got = Vec::new();
    let mut buf = [0u8; 4096];
    while !until(&String::from_utf8_lossy(&got)) {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => got.extend_from_slice(&buf[..n]),
        }
    }
    String::from_utf8_lossy(&got).into_owned()
}

fn get(port: u16, path: &str, extra: &str) -> String {
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{extra}Connection: close\r\n\r\n"
    );
    exchange(port, &request, |_| false).1
}

fn header_of<'a>(response: &'a str, name: &str) -> Vec<&'a str> {
    let head = response.split("\r\n\r\n").next().unwrap_or_default();
    head.lines()
        .filter_map(|l| l.split_once(':'))
        .filter(|(n, _)| n.trim().eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim())
        .collect()
}

/// The body of a raw response, its chunks joined when it came chunked.
fn body_of(response: &str) -> String {
    let (_, body) = response.split_once("\r\n\r\n").unwrap_or_default();
    let chunked = header_of(response, "transfer-encoding")
        .iter()
        .any(|v| v.eq_ignore_ascii_case("chunked"));
    if !chunked {
        return body.to_string();
    }
    let mut out = String::new();
    let mut rest = body;
    while let Some((size, after)) = rest.split_once("\r\n") {
        let size = usize::from_str_radix(size.trim(), 16).expect("chunk size");
        if size == 0 {
            break;
        }
        out.push_str(&after[..size]);
        rest = &after[size + 2..];
    }
    out
}

/// A page load in the frame, as a browser asks for one.
const FRAME_LOAD: &str = "Sec-Fetch-Dest: iframe\r\nAccept-Encoding: gzip, br\r\n";

fn proxied() -> (Fake, FrameProxy, Arc<Mutex<Seen>>) {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let fake = fake(Arc::clone(&seen));
    let proxy = FrameProxy::start(&format!("http://localhost:{}/", fake.port), 0).expect("proxy");
    (fake, proxy, seen)
}

#[test]
fn strips_only_the_headers_that_forbid_framing() {
    let (fake, proxy, seen) = proxied();
    let origin = format!("http://127.0.0.1:{}", proxy.port());
    let res = get(proxy.port(), "/", &format!("Origin: {origin}\r\n"));
    assert!(res.starts_with("HTTP/1.1 200"), "{res}");
    assert!(res.ends_with("<p>the site</p>"), "{res}");
    assert_eq!(
        header_of(&res, "content-security-policy"),
        ["default-src 'self'; img-src * data:"]
    );
    assert!(header_of(&res, "content-security-policy-report-only").is_empty());
    assert!(header_of(&res, "x-frame-options").is_empty());
    assert_eq!(header_of(&res, "x-powered-by"), ["a dev server"]);
    let seen = seen.lock().expect("seen");
    assert_eq!(
        seen.host.as_deref(),
        Some(&*format!("localhost:{}", fake.port))
    );
    assert_eq!(
        seen.origin.as_deref(),
        Some(&*format!("http://localhost:{}", fake.port))
    );
    assert_eq!(proxy.url(), format!("{origin}/"));
}

#[test]
fn streams_a_body_as_it_comes() {
    let (fake, proxy, _) = proxied();
    let request = "GET /stream HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n";
    // The second half is only sent once the first came through the proxy.
    let (mut stream, first) = exchange(proxy.port(), request, |s| s.contains("first-half"));
    assert!(first.contains("first-half"), "{first}");
    assert!(!first.contains("second-half"));
    fake.release
        .send(Bytes::from_static(b"second-half"))
        .expect("release");
    drop(fake.release);
    let rest = read_until(&mut stream, |s| s.ends_with("0\r\n\r\n"));
    assert!(rest.contains("second-half"), "{rest}");
    drop(fake.runtime);
}

#[test]
fn passes_a_websocket_through_both_ways() {
    let (_fake, proxy, _) = proxied();
    let request = "GET /hmr HTTP/1.1\r\nHost: x\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\
Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n";
    let (mut stream, head) = exchange(proxy.port(), request, |s| s.contains("\r\n\r\n"));
    assert!(head.starts_with("HTTP/1.1 101"), "{head}");
    assert_eq!(header_of(&head, "upgrade"), ["websocket"]);
    for word in ["ping", "hot update"] {
        stream.write_all(word.as_bytes()).expect("write");
        let echo = read_until(&mut stream, |s| s.len() >= word.len());
        assert_eq!(echo, word);
    }
}

#[test]
fn points_redirects_on_the_dev_server_at_the_proxy() {
    let (_fake, proxy, _) = proxied();
    let res = get(proxy.port(), "/redirect", "");
    assert!(res.starts_with("HTTP/1.1 307"), "{res}");
    let to = format!("http://127.0.0.1:{}/next?a=1", proxy.port());
    assert_eq!(header_of(&res, "location"), [to.as_str()]);
    let away = get(proxy.port(), "/away", "");
    assert_eq!(header_of(&away, "location"), ["https://example.com/"]);
}

#[test]
fn forwards_only_to_the_dev_server_whatever_the_target_says() {
    let (_fake, proxy, seen) = proxied();
    let res = get(proxy.port(), "http://example.com/", "");
    assert!(res.ends_with("<p>the site</p>"), "{res}");
    let host = seen.lock().expect("seen").host.clone();
    assert!(host.is_some_and(|h| h.starts_with("localhost:")));
}

#[test]
fn says_so_when_the_dev_server_is_gone() {
    let port = crate::free_port().expect("port");
    let proxy = FrameProxy::start(&format!("http://127.0.0.1:{port}/"), 0).expect("proxy");
    let res = get(proxy.port(), "/", "");
    assert!(res.starts_with("HTTP/1.1 502"), "{res}");
    assert!(res.contains("could not reach the dev server"), "{res}");
}

#[test]
fn refuses_a_dev_server_off_this_machine() {
    for url in [
        "http://192.168.1.20:3000/",
        "http://example.com:3000/",
        "http://10.0.0.1/",
        "https://localhost:3000/",
        "http://user@localhost:3000/",
        "ftp://localhost:21/",
        "http://localhost.example.com:3000/",
    ] {
        let err = FrameProxy::start(url, 0).expect_err(url);
        assert!(matches!(err, CoreError::Invalid(_)), "{url}: {err}");
    }
    for url in [
        "http://localhost:3000/",
        "http://127.0.0.1:3000",
        "http://[::1]:3000/",
    ] {
        assert!(FrameProxy::start(url, 0).is_ok(), "{url}");
    }
}

#[test]
fn stops_when_dropped_and_tries_its_old_port_again() {
    let (_fake, proxy, _) = proxied();
    let port = proxy.port();
    drop(proxy);
    let deadline = Instant::now() + Duration::from_secs(5);
    while StdStream::connect((Ipv4Addr::LOCALHOST, port)).is_ok() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(StdStream::connect((Ipv4Addr::LOCALHOST, port)).is_err());
    let again = FrameProxy::start("http://localhost:3000/", port).expect("proxy");
    assert_eq!(again.port(), port);
}

#[test]
fn takes_frame_ancestors_out_of_a_policy_and_nothing_else() {
    let cases = [
        ("default-src 'self'", None),
        ("frame-ancestors 'none'", Some(None)),
        (
            "default-src 'self';frame-ancestors 'none' ;script-src 'unsafe-eval'",
            Some(Some("default-src 'self'; script-src 'unsafe-eval'")),
        ),
        (
            "FRAME-ANCESTORS https://a.test, img-src *; frame-ancestors 'self'",
            Some(Some("img-src *")),
        ),
        ("frame-src 'self'", None),
    ];
    for (policy, want) in cases {
        let got = without_frame_ancestors(policy);
        assert_eq!(got.as_ref().map(Option::as_deref), want, "{policy}");
    }
}

#[test]
fn knows_the_dev_server_under_each_loopback_name() {
    let up = Upstream::parse("http://localhost:3010/").expect("upstream");
    assert_eq!(up.authority(), "localhost:3010");
    assert_eq!(up.rest_of("http://127.0.0.1:3010/a?b"), Some("/a?b"));
    assert_eq!(up.rest_of("http://[::1]:3010"), Some(""));
    assert_eq!(up.rest_of("http://localhost:3011/"), None);
    assert_eq!(up.rest_of("http://example.com:3010/"), None);
    assert_eq!(up.rest_of("/relative"), None);
    let v6 = Upstream::parse("http://[::1]:5173").expect("v6");
    assert_eq!(v6.lookup_host(), "::1");
}

#[test]
fn puts_the_shim_tag_in_a_page_once_right_after_its_head() {
    let (_fake, proxy, seen) = proxied();
    let res = get(proxy.port(), "/page", FRAME_LOAD);
    assert!(res.starts_with("HTTP/1.1 200"), "{res}");
    let body = body_of(&res);
    assert_eq!(body.matches(TAG).count(), 1, "{body}");
    assert!(body.contains(&format!("<head>{TAG}<title>")), "{body}");
    assert_eq!(body.replace(TAG, ""), PAGE);
    // The length it came with is gone, so nothing reads a stale one.
    assert!(header_of(&res, "content-length").is_empty(), "{res}");
    assert_eq!(header_of(&res, "transfer-encoding"), ["chunked"]);
    // The page was asked for plain.
    assert_eq!(seen.lock().expect("seen").accept_encoding, None);
    // Only frame-ancestors leaves the policy; nothing is added to it.
    assert_eq!(
        header_of(&res, "content-security-policy"),
        ["default-src 'self'; img-src * data:"]
    );
}

#[test]
fn reads_a_page_that_comes_gzip_anyway() {
    let (_fake, proxy, _) = proxied();
    let res = get(proxy.port(), "/page.gz", FRAME_LOAD);
    assert!(header_of(&res, "content-encoding").is_empty(), "{res}");
    assert_eq!(
        body_of(&res),
        PAGE.replace("<head>", &format!("<head>{TAG}"))
    );
}

#[test]
fn leaves_everything_but_a_page_load_as_it_came() {
    let (_fake, proxy, seen) = proxied();
    // A script, even one asked for like a page.
    let script = get(proxy.port(), "/app.js", FRAME_LOAD);
    assert_eq!(header_of(&script, "content-length"), ["23"]);
    assert_eq!(body_of(&script), "<head>not a page</head>");
    // HTML a script fetches is not a page load.
    let fetched = get(
        proxy.port(),
        "/page",
        "Sec-Fetch-Dest: empty\r\nAccept-Encoding: gzip\r\n",
    );
    assert_eq!(body_of(&fetched), PAGE);
    assert_eq!(
        header_of(&fetched, "content-length"),
        [PAGE.len().to_string().as_str()]
    );
    assert_eq!(
        seen.lock().expect("seen").accept_encoding.as_deref(),
        Some("gzip")
    );
    // Without Sec-Fetch-Dest, a request that accepts HTML is a page load.
    let older = get(proxy.port(), "/page", "Accept: text/html,*/*\r\n");
    assert_eq!(body_of(&older).matches(TAG).count(), 1);
}

#[test]
fn streams_a_page_once_its_head_is_in() {
    let (fake, proxy, _) = proxied();
    let request =
        format!("GET /stream-page HTTP/1.1\r\nHost: x\r\n{FRAME_LOAD}Connection: close\r\n\r\n");
    // The rest of the page is only sent once its start came through.
    let (mut stream, first) = exchange(proxy.port(), &request, |s| s.contains("first-half"));
    assert!(
        first.contains(&format!("<head>{TAG}<title>first-half")),
        "{first}"
    );
    fake.release
        .send(Bytes::from_static(
            b"</head><body>second-half</body></html>",
        ))
        .expect("release");
    drop(fake.release);
    let rest = read_until(&mut stream, |s| s.ends_with("0\r\n\r\n"));
    assert!(rest.contains("second-half"), "{rest}");
    assert_eq!(format!("{first}{rest}").matches(TAG).count(), 1);
    drop(fake.runtime);
}

#[test]
fn serves_the_shim_itself_and_never_forwards_its_path() {
    let (_fake, proxy, seen) = proxied();
    let res = get(proxy.port(), "/__maquette/bridge.js?v=1", "");
    assert!(res.starts_with("HTTP/1.1 200"), "{res}");
    assert_eq!(
        header_of(&res, "content-type"),
        ["text/javascript; charset=utf-8"]
    );
    assert_eq!(header_of(&res, "cache-control"), ["no-store"]);
    assert_eq!(body_of(&res), SHIM);
    let other = get(proxy.port(), "/__maquette/anything", "");
    assert!(other.starts_with("HTTP/1.1 404"), "{other}");
    assert!(seen.lock().expect("seen").path.is_none());
}

#[test]
fn finds_where_the_tag_goes() {
    // Each page start, and what follows the tag in it (None: not yet known).
    let cases = [
        (
            "<!doctype html><html><head><meta charset=utf-8>",
            Some("<meta charset=utf-8>"),
        ),
        ("<html><HEAD lang=\"en\">\n<title>", Some("\n<title>")),
        ("<html><header>a</header></head>x", Some("</head>x")),
        ("<html><body class=\"a\"><p>", Some("<p>")),
        ("<html><head", None),
        ("<html><head data-x=\"1\"", None),
        ("<p>no structure at all</p>", None),
    ];
    for (html, after) in cases {
        assert_eq!(spot(html.as_bytes()).map(|i| &html[i..]), after, "{html}");
    }
}
