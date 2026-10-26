//! The frame proxy: a small reverse proxy on 127.0.0.1 in front of the dev
//! server the Page view shows, so a site that forbids framing (most do, with
//! `X-Frame-Options: DENY` or `frame-ancestors 'none'`) still shows in the
//! view's iframe without editing the site.
//!
//! It forwards every request to the one loopback dev server it was made
//! for, never anywhere else: the request's own target is cut to its path
//! and query. Bodies stream through untouched in both directions, except
//! that a page gets the page bridge shim's script tag ([`inject`]), which
//! the proxy serves itself under `/__maquette/` and never forwards.
//! WebSocket upgrades (the HMR of Next, Vite and Astro) are spliced
//! through, and [`headers`] says which headers change. It runs on its own
//! small tokio runtime, and dropping it stops it and closes every
//! connection.

use std::convert::Infallible;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use http_body_util::{Either, Full};
use hyper::body::{Bytes, Incoming};
use hyper::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderValue, X_CONTENT_TYPE_OPTIONS};
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

mod headers;
mod inject;
pub use headers::Upstream;
use headers::{fix_request, fix_response, wants_upgrade};
use inject::Injected;

use crate::error::{CoreError, CoreResult};

type Body = Either<Incoming, Either<Full<Bytes>, Injected>>;

/// How long a stop waits for open connections to end.
const STOP_TIMEOUT: Duration = Duration::from_millis(500);

/// A running frame proxy. Dropping it stops it.
pub struct FrameProxy {
    url: String,
    addr: SocketAddr,
    runtime: Option<tokio::runtime::Runtime>,
}

impl std::fmt::Debug for FrameProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameProxy")
            .field("url", &self.url)
            .finish()
    }
}

/// What every connection shares: where to forward and the proxy's origin.
struct Route {
    upstream: Upstream,
    /// `http://127.0.0.1:<port>`, no slash.
    origin: String,
}

impl FrameProxy {
    /// Starts a proxy for the dev server at `upstream` on 127.0.0.1:`port`,
    /// or on a free port when `port` is 0 or taken.
    ///
    /// # Errors
    ///
    /// Refuses a dev server that is not plain `http` on this machine, and
    /// fails when no port can be bound or the runtime cannot start.
    pub fn start(upstream: &str, port: u16) -> CoreResult<Self> {
        let upstream = Upstream::parse(upstream)?;
        let unavailable = |err: io::Error| {
            CoreError::Unavailable(format!("Maquette could not start its frame proxy. {err}"))
        };
        let listener = bind(port).map_err(unavailable)?;
        listener.set_nonblocking(true).map_err(unavailable)?;
        let addr = listener.local_addr().map_err(unavailable)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("maquette-frame")
            .enable_all()
            .build()
            .map_err(unavailable)?;
        let listener = {
            let _entered = runtime.enter();
            tokio::net::TcpListener::from_std(listener).map_err(unavailable)?
        };
        let origin = format!("http://127.0.0.1:{}", addr.port());
        let route = Arc::new(Route {
            upstream,
            origin: origin.clone(),
        });
        runtime.spawn(accept(listener, route));
        Ok(Self {
            url: format!("{origin}/"),
            addr,
            runtime: Some(runtime),
        })
    }

    /// `http://127.0.0.1:<port>/`: what the frame loads.
    pub fn url(&self) -> &str {
        &self.url
    }

    pub const fn port(&self) -> u16 {
        self.addr.port()
    }
}

impl Drop for FrameProxy {
    fn drop(&mut self) {
        let Some(runtime) = self.runtime.take() else {
            return;
        };
        // A runtime cannot shut down from inside another one, and the
        // caller may be an async command, so this happens on its own thread.
        let stopped = std::thread::Builder::new()
            .name("maquette-frame-stop".into())
            .spawn(move || runtime.shutdown_timeout(STOP_TIMEOUT))
            .map(std::thread::JoinHandle::join);
        if !matches!(stopped, Ok(Ok(()))) {
            tracing::warn!("the frame proxy did not stop cleanly");
        }
    }
}

fn bind(port: u16) -> io::Result<std::net::TcpListener> {
    if port != 0
        && let Ok(listener) = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port))
    {
        return Ok(listener);
    }
    std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
}

async fn accept(listener: tokio::net::TcpListener, route: Arc<Route>) {
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(err) => {
                tracing::debug!(error = %err, "the frame proxy could not accept a connection");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let _ = stream.set_nodelay(true);
        let route = Arc::clone(&route);
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |req| {
                let route = Arc::clone(&route);
                async move { Ok::<_, Infallible>(forward(req, &route).await) }
            });
            let served = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .with_upgrades()
                .await;
            if let Err(err) = served {
                tracing::debug!(error = %err, "a frame proxy connection ended with an error");
            }
        });
    }
}

/// Sends `req` to the dev server and hands back its answer, fixed.
async fn forward(mut req: Request<Incoming>, route: &Route) -> Response<Body> {
    if req.uri().path().starts_with(inject::RESERVED) {
        return reserved(&req);
    }
    let upgrade = wants_upgrade(req.headers());
    let from_frame = upgrade.then(|| hyper::upgrade::on(&mut req));
    let page = inject::wants_page(req.method(), req.headers());
    fix_request(req.headers_mut(), &route.upstream, &route.origin, upgrade);
    if page {
        inject::fix_page_request(req.headers_mut());
    }
    // Only the path and query go on: the target is always the dev server.
    let target = req
        .uri()
        .path_and_query()
        .and_then(|p| Uri::try_from(p.as_str()).ok())
        .unwrap_or_else(|| Uri::from_static("/"));
    *req.uri_mut() = target;
    let mut sender = match connect(&route.upstream).await {
        Ok(sender) => sender,
        Err(err) => return unreachable(&route.upstream, &err.to_string()),
    };
    let mut res = match sender.send_request(req).await {
        Ok(res) => res,
        Err(err) => return unreachable(&route.upstream, &err.to_string()),
    };
    let switched = res.status() == StatusCode::SWITCHING_PROTOCOLS;
    if let (true, Some(from_frame)) = (switched, from_frame) {
        let from_server = hyper::upgrade::on(&mut res);
        tokio::spawn(async move {
            let (Ok(frame), Ok(server)) = tokio::join!(from_frame, from_server) else {
                return;
            };
            let (mut frame, mut server) = (TokioIo::new(frame), TokioIo::new(server));
            let _ = tokio::io::copy_bidirectional(&mut frame, &mut server).await;
        });
    }
    fix_response(res.headers_mut(), &route.upstream, &route.origin, switched);
    if let Some(decode) = page
        .then(|| inject::page_body(res.status(), res.headers()))
        .flatten()
    {
        inject::fix_page_response(res.headers_mut());
        return res.map(|body| Either::Right(Either::Right(Injected::new(body, decode))));
    }
    res.map(Either::Left)
}

/// What the proxy answers itself under `/__maquette/`: the shim, else 404.
fn reserved(req: &Request<Incoming>) -> Response<Body> {
    let shim = req.uri().path() == inject::SHIM_PATH
        && matches!(*req.method(), Method::GET | Method::HEAD);
    let (status, body, kind) = if shim {
        (
            StatusCode::OK,
            inject::SHIM,
            "text/javascript; charset=utf-8",
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            "Not found.",
            "text/plain; charset=utf-8",
        )
    };
    let mut response = plain(status, body.to_string(), kind);
    let headers = response.headers_mut();
    // Each app update may bring a new shim.
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    response
}

fn plain(status: StatusCode, text: String, kind: &'static str) -> Response<Body> {
    let mut response = Response::new(Either::Right(Either::Left(Full::new(Bytes::from(text)))));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(kind));
    response
}

/// A fresh connection to the dev server, at a loopback address only.
async fn connect(
    upstream: &Upstream,
) -> io::Result<hyper::client::conn::http1::SendRequest<Incoming>> {
    let mut last = None;
    let mut stream = None;
    for addr in tokio::net::lookup_host((upstream.lookup_host(), upstream.port())).await? {
        if !addr.ip().is_loopback() {
            continue;
        }
        match TcpStream::connect(addr).await {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(err) => last = Some(err),
        }
    }
    let stream = stream.ok_or_else(|| {
        last.unwrap_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no loopback address"))
    })?;
    let _ = stream.set_nodelay(true);
    let (sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .map_err(io::Error::other)?;
    tokio::spawn(async move {
        if let Err(err) = conn.with_upgrades().await {
            tracing::debug!(error = %err, "a dev server connection ended with an error");
        }
    });
    Ok(sender)
}

fn unreachable(upstream: &Upstream, why: &str) -> Response<Body> {
    let text = format!(
        "Maquette could not reach the dev server at {}. {why}",
        upstream.origin()
    );
    plain(StatusCode::BAD_GATEWAY, text, "text/plain; charset=utf-8")
}

#[cfg(test)]
mod tests;
