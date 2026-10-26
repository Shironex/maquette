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
    };
    let mut res = match req.uri().path() {
        "/stream" => {
            let (tx, rx) = mpsc::unbounded_channel();
            let _ = tx.send(Bytes::from_static(b"first-half "));
            if let Some(mut later) = held.lock().await.take() {
                tokio::spawn(async move {
                    while let Some(chunk) = later.recv().await {
                        let _ = tx.send(chunk);
                    }
                });
            }
            Response::new(Chunks(rx).boxed())
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
    let port = crate::site::preview::free_port().expect("port");
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
