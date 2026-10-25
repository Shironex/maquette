//! A tiny HTTP/1.1 server for the provider tests: one thread, one request
//! per connection (`Connection: close`), a handler closure that answers
//! from the method and path. Nothing leaves 127.0.0.1, so no test can
//! reach Tripo or Meshy or spend a credit.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use serde_json::Value;

#[derive(Debug, Clone)]
pub(super) struct Request {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub(super) fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

pub(super) struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Reply {
    pub(super) fn json(status: u16, body: &Value) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: body.to_string().into_bytes(),
        }
    }

    pub(super) fn ok(body: &Value) -> Self {
        Self::json(200, body)
    }

    pub(super) fn bytes(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "model/gltf-binary".into())],
            body,
        }
    }

    pub(super) fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

type Handler = dyn Fn(&str, &Request) -> Reply + Send + Sync;

pub(super) struct FakeServer {
    pub url: String,
    log: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeServer {
    /// Starts a server; `handler` gets the base URL (for download links)
    /// and the request.
    pub(super) fn start(handler: impl Fn(&str, &Request) -> Reply + Send + Sync + 'static) -> Self {
        Self::start_at("127.0.0.1:0", handler)
    }

    /// As [`FakeServer::start`], on a chosen address: a test can let a
    /// client fail to connect first, then start the server where it looks.
    pub(super) fn start_at(
        addr: &str,
        handler: impl Fn(&str, &Request) -> Reply + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind(addr).expect("bind");
        let url = format!("http://{}", listener.local_addr().expect("addr"));
        let log = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let handler: Arc<Handler> = Arc::new(handler);
        let thread = {
            let (log, stop, base) = (Arc::clone(&log), Arc::clone(&stop), url.clone());
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let (log, handler, base) =
                        (Arc::clone(&log), Arc::clone(&handler), base.clone());
                    std::thread::spawn(move || serve(stream, &base, &*handler, &log));
                }
            })
        };
        Self {
            url,
            log,
            stop,
            thread: Some(thread),
        }
    }

    pub(super) fn requests(&self) -> Vec<Request> {
        self.log.lock().expect("log").clone()
    }

    /// How many requests matched `method` and started with `path`.
    pub(super) fn count(&self, method: &str, path: &str) -> usize {
        self.requests()
            .iter()
            .filter(|r| r.method == method && r.path.starts_with(path))
            .count()
    }

    pub(super) fn find(&self, method: &str, path: &str) -> Option<Request> {
        self.requests()
            .into_iter()
            .find(|r| r.method == method && r.path == path)
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop.
        let _ = TcpStream::connect(self.url.trim_start_matches("http://"));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(stream: TcpStream, base: &str, handler: &Handler, log: &Mutex<Vec<Request>>) {
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        return;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).is_err() {
            return;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((k, v)) = header.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    let get = |name: &str| {
        headers
            .iter()
            .find(|(k, _): &&(String, String)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
    };
    let mut body = Vec::new();
    if let Some(len) = get("content-length").and_then(|v| v.parse::<usize>().ok()) {
        body.resize(len, 0);
        if reader.read_exact(&mut body).is_err() {
            return;
        }
    } else if get("transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
        loop {
            let mut size = String::new();
            if reader.read_line(&mut size).is_err() {
                return;
            }
            let Ok(n) = usize::from_str_radix(size.trim(), 16) else {
                return;
            };
            let mut chunk = vec![0; n + 2];
            if reader.read_exact(&mut chunk).is_err() {
                return;
            }
            if n == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..n]);
        }
    }
    let request = Request {
        method,
        path,
        headers,
        body,
    };
    let reply = handler(base, &request);
    log.lock().expect("log").push(request);
    let mut out = stream;
    let mut head = format!(
        "HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    );
    for (k, v) in &reply.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    let _ = out.write_all(head.as_bytes());
    let _ = out.write_all(&reply.body);
    let _ = out.flush();
}
