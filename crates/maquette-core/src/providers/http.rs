//! The blocking HTTP client the provider calls share: bearer auth, an
//! injectable base URL (the tests point it at a fake server), 429 back off
//! that honours `Retry-After`, and sleeps a cancel or app exit can cut
//! short.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use reqwest::blocking::{Client, RequestBuilder, Response};
use serde_json::Value;

/// reqwest is built without a bundled TLS backend choice, so the process
/// picks one: ring, which builds the same on macOS and Windows. Safe to
/// call any number of times; the first call wins.
pub fn ensure_crypto() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        // Err means another provider is already installed, which is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// How often to poll and how long to back off. The tests shrink these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// Between two status checks of a running task.
    pub poll: Duration,
    /// Never poll faster than this, whatever a server asks.
    pub min_poll: Duration,
    /// The wait after a 429 without a `Retry-After` header.
    pub backoff: Duration,
    /// The longest a `Retry-After` may make a job wait at once.
    pub max_backoff: Duration,
    /// 429 answers in a row before the job gives up.
    pub max_retries: u32,
    /// The longest a request may take.
    pub request_timeout: Duration,
}

impl Timing {
    /// Tripo: every 2 s.
    pub const TRIPO: Self = Self {
        poll: Duration::from_secs(2),
        min_poll: Duration::from_secs(1),
        backoff: Duration::from_secs(5),
        max_backoff: Duration::from_secs(120),
        max_retries: 8,
        request_timeout: Duration::from_secs(120),
    };

    /// Meshy: every 5 s unless `Retry-After` says otherwise.
    pub const MESHY: Self = Self {
        poll: Duration::from_secs(5),
        ..Self::TRIPO
    };

    /// Fast enough for tests against a local fake server.
    pub const fn instant() -> Self {
        Self {
            poll: Duration::from_millis(5),
            min_poll: Duration::from_millis(1),
            backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(50),
            max_retries: 3,
            request_timeout: Duration::from_secs(10),
        }
    }

    /// Clamps a server's wish to this timing's floor and ceiling.
    pub fn clamp(&self, wait: Duration) -> Duration {
        wait.max(self.min_poll).min(self.max_backoff)
    }
}

/// Why a provider call failed, in terms the job turns into a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// 401: the key is wrong or revoked.
    Auth,
    /// 402, or Tripo's 403 with code 2010: out of credits or on a plan
    /// without API access.
    Credits(String),
    /// Still 429 after every retry.
    RateLimited,
    /// 409: Meshy has started the task, so it cannot be cancelled.
    Conflict(String),
    /// 403, 404 or 410 on a download: the link expired.
    Expired,
    /// Any other answer the provider gave.
    Status { status: u16, message: String },
    /// No answer: offline, DNS, TLS or a timeout.
    Network(String),
    /// The answer was not what the API documents.
    Malformed(String),
    /// Cancelled or the app is closing.
    Stopped,
}

impl ApiError {
    /// A sentence for the queue and the Board.
    pub fn message(&self, service: &str) -> String {
        match self {
            Self::Auth => format!(
                "{service} did not accept the API key. Check it in Settings > 3D providers."
            ),
            Self::Credits(detail) => {
                if detail.is_empty() {
                    format!("{service} says the account is out of credits.")
                } else {
                    format!("{service} says the account is out of credits: {detail}")
                }
            }
            Self::RateLimited => {
                format!("{service} kept asking to slow down. Try again in a few minutes.")
            }
            Self::Conflict(detail) => detail.clone(),
            Self::Expired => format!("The {service} download link expired."),
            Self::Status { status, message } => {
                if message.is_empty() {
                    format!("{service} answered {status}.")
                } else {
                    format!("{service} answered {status}: {message}")
                }
            }
            Self::Network(detail) => format!("Could not reach {service}: {detail}"),
            Self::Malformed(detail) => {
                format!("{service} sent an answer I could not read: {detail}")
            }
            Self::Stopped => "Stopped.".to_string(),
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

/// A cancel or exit flag. Sleeps check it every few milliseconds.
#[derive(Debug, Default)]
pub struct StopFlag(AtomicBool);

impl StopFlag {
    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Sleeps for `wait`, or less when the flag goes up. Err when stopped.
    pub fn sleep(&self, wait: Duration) -> ApiResult<()> {
        let end = Instant::now() + wait;
        loop {
            if self.is_stopped() {
                return Err(ApiError::Stopped);
            }
            let now = Instant::now();
            if now >= end {
                return Ok(());
            }
            std::thread::sleep((end - now).min(Duration::from_millis(20)));
        }
    }
}

/// One service's API: its base URL, key and timing.
pub struct Api {
    client: Client,
    base: String,
    key: String,
    pub timing: Timing,
    /// "Tripo" or "Meshy", for messages.
    pub service: &'static str,
}

impl Api {
    pub fn new(service: &'static str, base: &str, key: &str, timing: Timing) -> ApiResult<Self> {
        ensure_crypto();
        let client = Client::builder()
            .timeout(timing.request_timeout)
            .user_agent(concat!("Maquette/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ApiError::Network(e.to_string()))?;
        Ok(Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            key: key.trim().to_string(),
            timing,
            service,
        })
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base, path.trim_start_matches('/'))
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(self.url(path)).bearer_auth(&self.key)
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        self.client.post(self.url(path)).bearer_auth(&self.key)
    }

    /// A POST with a JSON body.
    pub fn post_json(&self, path: &str, body: &Value) -> RequestBuilder {
        self.post(path)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
    }

    pub fn delete(&self, path: &str) -> RequestBuilder {
        self.client.delete(self.url(path)).bearer_auth(&self.key)
    }

    /// Sends a request built by `build`, retrying 429 answers after the
    /// wait the server asks for. `build` runs once per attempt because a
    /// multipart body cannot be sent twice.
    pub fn send(
        &self,
        stop: &StopFlag,
        build: impl Fn() -> ApiResult<RequestBuilder>,
    ) -> ApiResult<Response> {
        let mut attempt = 0;
        loop {
            if stop.is_stopped() {
                return Err(ApiError::Stopped);
            }
            let response = build()?
                .send()
                .map_err(|e| ApiError::Network(network_detail(e)))?;
            if response.status() != StatusCode::TOO_MANY_REQUESTS {
                return Ok(response);
            }
            attempt += 1;
            if attempt > self.timing.max_retries {
                return Err(ApiError::RateLimited);
            }
            let wait = retry_after(&response).unwrap_or_else(|| {
                // Exponential without a header: 1x, 2x, 4x the base wait.
                self.timing.backoff * 2u32.saturating_pow(attempt - 1).min(16)
            });
            tracing::info!(
                service = self.service,
                attempt,
                ?wait,
                "rate limited, backing off"
            );
            stop.sleep(self.timing.clamp(wait))?;
        }
    }

    /// Sends and reads a JSON answer, mapping error statuses.
    pub fn json(
        &self,
        stop: &StopFlag,
        build: impl Fn() -> ApiResult<RequestBuilder>,
    ) -> ApiResult<Value> {
        let response = self.send(stop, build)?;
        read_json(response)
    }

    /// As [`Api::json`], plus the `Retry-After` the answer carried, which
    /// Meshy sends with a task that is still running.
    pub fn json_wait(
        &self,
        stop: &StopFlag,
        build: impl Fn() -> ApiResult<RequestBuilder>,
    ) -> ApiResult<(Value, Option<Duration>)> {
        let response = self.send(stop, build)?;
        let wait = retry_after(&response);
        read_json(response).map(|body| (body, wait))
    }

    /// Downloads a file, without the API key: result links are signed.
    pub fn download(&self, stop: &StopFlag, url: &str) -> ApiResult<Vec<u8>> {
        if stop.is_stopped() {
            return Err(ApiError::Stopped);
        }
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|e| ApiError::Network(network_detail(e)))?;
        let status = response.status();
        if matches!(status.as_u16(), 403 | 404 | 410) {
            return Err(ApiError::Expired);
        }
        if !status.is_success() {
            return Err(ApiError::Status {
                status: status.as_u16(),
                message: String::new(),
            });
        }
        response
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| ApiError::Network(network_detail(e)))
    }
}

/// The error chain without the URL, which can carry signed query strings.
fn network_detail(error: reqwest::Error) -> String {
    let error = error.without_url();
    if error.is_timeout() {
        return "the request timed out.".to_string();
    }
    if error.is_connect() {
        return "the connection failed. Check the network.".to_string();
    }
    let mut text = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(inner) = source {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        source = inner.source();
    }
    text
}

/// Seconds from a `Retry-After` header. HTTP dates are not used by either
/// service, so they count as absent.
pub fn retry_after(response: &Response) -> Option<Duration> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(Duration::from_secs_f64)
}

fn read_json(response: Response) -> ApiResult<Value> {
    let status = response.status();
    let text = response
        .text()
        .map_err(|e| ApiError::Network(network_detail(e)))?;
    let body: Option<Value> = serde_json::from_str(&text).ok();
    if status.is_success() {
        return body.ok_or_else(|| ApiError::Malformed("the answer was not JSON.".to_string()));
    }
    let message = body
        .as_ref()
        .and_then(|b| b.get("message"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let code = body
        .as_ref()
        .and_then(|b| b.get("code"))
        .and_then(Value::as_i64);
    Err(match status.as_u16() {
        401 => ApiError::Auth,
        402 => ApiError::Credits(message),
        403 if code == Some(2010) => ApiError::Credits(message),
        409 => ApiError::Conflict(message),
        status => ApiError::Status { status, message },
    })
}
