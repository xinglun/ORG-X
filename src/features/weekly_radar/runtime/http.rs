//! Injected synchronous HTTP boundary for Weekly Radar adapters.

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::error::RuntimeError;

/// Maximum HTTP response body size retained by the runtime transport.
pub const MAX_HTTP_RESPONSE_BODY_BYTES: usize = 1_048_576;

/// A complete response returned by an injected HTTP client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl HttpResponse {
    /// Creates a response with no headers.
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    /// Creates a successful 200 response with no headers.
    pub fn ok(body: impl Into<String>) -> Self {
        Self::new(200, body)
    }

    /// Adds one response header and returns the updated response.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Returns the HTTP status code.
    pub const fn status(&self) -> u16 {
        self.status
    }

    /// Returns all response headers in transport order.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Returns the response body as UTF-8 text.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Returns whether the status is in the 2xx range.
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

/// Provider-neutral synchronous HTTP port used by all runtime adapters.
pub trait HttpClient {
    /// Performs one GET request with the supplied headers.
    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, RuntimeError>;

    /// Performs one GET request with a caller-selected finite body limit.
    ///
    /// Implementations may keep the default limit when a specialized limit is
    /// not supported. Runtime adapters use this only for source-specific
    /// payloads whose documented size envelope is larger than the default.
    fn get_with_max_body_bytes(
        &self,
        url: &str,
        headers: &[(String, String)],
        _max_body_bytes: usize,
    ) -> Result<HttpResponse, RuntimeError> {
        self.get(url, headers)
    }
}

/// One request captured by the in-memory fixture client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureRequest {
    url: String,
    headers: Vec<(String, String)>,
}

impl FixtureRequest {
    /// Returns the requested URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the headers supplied to the fixture request.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }
}

#[derive(Debug, Default)]
struct FixtureState {
    responses: BTreeMap<String, HttpResponse>,
    requests: Vec<FixtureRequest>,
}

/// In-memory HTTP client for deterministic fixture-driven tests.
#[derive(Clone, Debug, Default)]
pub struct FixtureHttpClient {
    state: Arc<Mutex<FixtureState>>,
}

impl FixtureHttpClient {
    /// Creates an empty fixture transport.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a fixture transport containing one response.
    pub fn with_response(url: impl Into<String>, response: HttpResponse) -> Self {
        let client = Self::new();
        client.insert(url, response);
        client
    }

    /// Adds or replaces the response returned for an exact URL.
    pub fn insert(&self, url: impl Into<String>, response: HttpResponse) {
        if let Ok(mut state) = self.state.lock() {
            state.responses.insert(url.into(), response);
        }
    }

    /// Returns all requests captured so far in request order.
    pub fn requests(&self) -> Vec<FixtureRequest> {
        self.state
            .lock()
            .map(|state| state.requests.clone())
            .unwrap_or_default()
    }
}

impl HttpClient for FixtureHttpClient {
    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, RuntimeError> {
        self.get_with_max_body_bytes(url, headers, MAX_HTTP_RESPONSE_BODY_BYTES)
    }

    fn get_with_max_body_bytes(
        &self,
        url: &str,
        headers: &[(String, String)],
        max_body_bytes: usize,
    ) -> Result<HttpResponse, RuntimeError> {
        let mut state = self.state.lock().map_err(|_| RuntimeError::FixtureState)?;
        state.requests.push(FixtureRequest {
            url: url.to_owned(),
            headers: headers.to_vec(),
        });
        let response = state
            .responses
            .get(url)
            .cloned()
            .ok_or(RuntimeError::FixtureMissing)?;
        if response.body().len() > max_body_bytes {
            return Err(RuntimeError::HttpResponseTooLarge);
        }
        Ok(response)
    }
}

/// Finite timeout budget applied to every production HTTP request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpTimeouts {
    connect: Duration,
    read: Duration,
    write: Duration,
    overall: Duration,
}

impl HttpTimeouts {
    /// Creates a timeout budget from finite phase and overall durations.
    pub const fn new(
        connect: Duration,
        read: Duration,
        write: Duration,
        overall: Duration,
    ) -> Self {
        Self {
            connect,
            read,
            write,
            overall,
        }
    }

    /// Returns the connection timeout.
    pub const fn connect(self) -> Duration {
        self.connect
    }

    /// Returns the response-read timeout.
    pub const fn read(self) -> Duration {
        self.read
    }

    /// Returns the request-write timeout.
    pub const fn write(self) -> Duration {
        self.write
    }

    /// Returns the overall request deadline.
    pub const fn overall(self) -> Duration {
        self.overall
    }
}

impl Default for HttpTimeouts {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(5),
            Duration::from_secs(30),
            Duration::from_secs(30),
            Duration::from_secs(60),
        )
    }
}

/// Production synchronous HTTP implementation backed by `ureq`.
#[derive(Clone, Debug)]
pub struct UreqHttpClient {
    agent: ureq::Agent,
    timeouts: HttpTimeouts,
}

impl UreqHttpClient {
    /// Creates a production HTTP client with a reusable connection agent.
    pub fn new() -> Self {
        Self::with_timeouts(HttpTimeouts::default())
    }

    /// Creates a production client with an explicit finite timeout budget.
    pub fn with_timeouts(timeouts: HttpTimeouts) -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(timeouts.connect())
                .timeout_read(timeouts.read())
                .timeout_write(timeouts.write())
                .timeout(timeouts.overall())
                .redirects(0)
                .build(),
            timeouts,
        }
    }

    /// Returns the timeout budget configured on this client.
    pub const fn timeouts(&self) -> HttpTimeouts {
        self.timeouts
    }
}

impl Default for UreqHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for UreqHttpClient {
    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, RuntimeError> {
        self.get_with_max_body_bytes(url, headers, MAX_HTTP_RESPONSE_BODY_BYTES)
    }

    fn get_with_max_body_bytes(
        &self,
        url: &str,
        headers: &[(String, String)],
        max_body_bytes: usize,
    ) -> Result<HttpResponse, RuntimeError> {
        let mut request = self.agent.get(url);
        for (name, value) in headers {
            request = request.set(name, value);
        }

        let response = match request.call() {
            Ok(response) => response,
            // HTTP status codes are data at this boundary. Adapters classify
            // them after receiving the same HttpResponse as fixture clients.
            Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(_)) => {
                return Err(RuntimeError::HttpRequest);
            }
        };
        let status = response.status();
        let headers = response
            .headers_names()
            .into_iter()
            .filter_map(|name| response.header(&name).map(|value| (name, value.to_owned())))
            .collect();
        let mut body_bytes = Vec::with_capacity(max_body_bytes.saturating_add(1));
        response
            .into_reader()
            .take(max_body_bytes.saturating_add(1) as u64)
            .read_to_end(&mut body_bytes)
            .map_err(|_| RuntimeError::HttpResponse)?;
        if body_bytes.len() > max_body_bytes {
            return Err(RuntimeError::HttpResponseTooLarge);
        }
        let body = String::from_utf8(body_bytes).map_err(|_| RuntimeError::HttpResponse)?;

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}
