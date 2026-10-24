//! The HTTP side: Streamable HTTP on 127.0.0.1 only, at `/mcp`, behind a
//! bearer token. Runs on its own small tokio runtime, so starting and
//! stopping it never touches the app's.

use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use http_body_util::{BodyExt as _, Full, combinators::BoxBody};
use hyper::body::{Bytes, Incoming};
use hyper::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue, WWW_AUTHENTICATE};
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;
use tower_service::Service as _;

use super::host::Host;
use super::hub::{CallOutcome, Hub};
use super::tools::McpTools;

pub const PATH: &str = "/mcp";

type Body = BoxBody<Bytes, Infallible>;
type Mcp = StreamableHttpService<McpTools, LocalSessionManager>;

/// A server that is listening. Dropping it does not stop it; call `stop`.
pub struct Running {
    pub addr: SocketAddr,
    cancel: CancellationToken,
    runtime: Option<tokio::runtime::Runtime>,
}

impl Running {
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Stops listening and ends every session. Blocks for up to a second.
    pub fn stop(mut self) {
        self.cancel.cancel();
        if let Some(runtime) = self.runtime.take() {
            // A runtime cannot shut down from inside another one, and the
            // caller may be an async command, so this happens on its own
            // thread.
            let joined = std::thread::Builder::new()
                .name("maquette-mcp-stop".into())
                .spawn(move || runtime.shutdown_timeout(Duration::from_secs(1)))
                .map(std::thread::JoinHandle::join);
            if joined.is_err() {
                tracing::warn!("the MCP runtime did not stop cleanly");
            }
        }
    }
}

/// Binds 127.0.0.1:`port` (0 picks a free one) and starts serving.
///
/// # Errors
///
/// Fails when the port is taken or the runtime cannot start.
pub fn start(
    port: u16,
    token: String,
    host: Arc<dyn Host>,
    hub: Arc<Hub>,
) -> std::io::Result<Running> {
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("maquette-mcp")
        .enable_all()
        .build()?;
    let cancel = CancellationToken::new();
    let config = StreamableHttpServerConfig::default()
        .with_cancellation_token(cancel.child_token())
        .with_sse_keep_alive(Some(Duration::from_secs(15)));
    let tools_hub = Arc::clone(&hub);
    let service: Mcp = StreamableHttpService::new(
        move || Ok(McpTools::new(Arc::clone(&host), Arc::clone(&tools_hub))),
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let listener = {
        let _entered = runtime.enter();
        tokio::net::TcpListener::from_std(listener)?
    };
    let token: Arc<str> = token.into();
    runtime.spawn(accept(listener, service, token, hub, cancel.clone()));
    Ok(Running {
        addr,
        cancel,
        runtime: Some(runtime),
    })
}

async fn accept(
    listener: tokio::net::TcpListener,
    service: Mcp,
    token: Arc<str>,
    hub: Arc<Hub>,
    cancel: CancellationToken,
) {
    loop {
        let stream = tokio::select! {
            () = cancel.cancelled() => break,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => stream,
                Err(err) => {
                    tracing::warn!(error = %err, "the MCP server could not accept a connection");
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }
            },
        };
        let service = service.clone();
        let token = Arc::clone(&token);
        let hub = Arc::clone(&hub);
        let cancel = cancel.clone();
        tokio::spawn(async move {
            let handler = hyper::service::service_fn(move |req| {
                let service = service.clone();
                let token = Arc::clone(&token);
                let hub = Arc::clone(&hub);
                async move { Ok::<_, Infallible>(route(req, service, &token, &hub).await) }
            });
            let conn = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), handler);
            tokio::select! {
                () = cancel.cancelled() => {}
                done = conn => if let Err(err) = done {
                    tracing::debug!(error = %err, "an MCP connection ended with an error");
                },
            }
        });
    }
}

fn plain(status: StatusCode, text: &'static str) -> Response<Body> {
    let mut response = Response::new(Full::new(Bytes::from_static(text.as_bytes())).boxed());
    *response.status_mut() = status;
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    response
}

/// Compares in constant time, so the token cannot be guessed byte by byte.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub(crate) fn authorized(header: Option<&HeaderValue>, token: &str) -> bool {
    let Some(value) = header.and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Some((scheme, given)) = value.trim().split_once(' ') else {
        return false;
    };
    scheme.eq_ignore_ascii_case("bearer") && same(given.trim().as_bytes(), token.as_bytes())
}

async fn route(req: Request<Incoming>, mut service: Mcp, token: &str, hub: &Hub) -> Response<Body> {
    if req.uri().path() != PATH {
        return plain(StatusCode::NOT_FOUND, "Maquette serves MCP at /mcp only.\n");
    }
    if !authorized(req.headers().get(AUTHORIZATION), token) {
        hub.log_call(
            "connect",
            String::new(),
            CallOutcome::Refused,
            Some("A request came without the right token.".to_string()),
            0.0,
        );
        let mut response = plain(
            StatusCode::UNAUTHORIZED,
            "Send the token from Maquette's Settings > Agents as a Bearer token.\n",
        );
        response.headers_mut().insert(
            WWW_AUTHENTICATE,
            HeaderValue::from_static("Bearer realm=\"maquette\""),
        );
        return response;
    }
    match service.call(req).await {
        Ok(response) => response,
        Err(never) => match never {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_exact_bearer_token_passes() {
        let ok = HeaderValue::from_static("Bearer abc123");
        assert!(authorized(Some(&ok), "abc123"));
        let lower = HeaderValue::from_static("bearer abc123");
        assert!(authorized(Some(&lower), "abc123"));
        for bad in [
            "Bearer abc124",
            "Bearer abc12",
            "Basic abc123",
            "abc123",
            "",
        ] {
            let value = HeaderValue::from_str(bad).expect("header");
            assert!(!authorized(Some(&value), "abc123"), "{bad}");
        }
        assert!(!authorized(None, "abc123"));
    }
}
