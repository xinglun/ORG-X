//! Injected synchronous HTTP boundary for Weekly Radar adapters.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use super::error::RuntimeError;

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
        let mut state = self.state.lock().map_err(|_| RuntimeError::FixtureState)?;
        state.requests.push(FixtureRequest {
            url: url.to_owned(),
            headers: headers.to_vec(),
        });
        state
            .responses
            .get(url)
            .cloned()
            .ok_or_else(|| RuntimeError::FixtureMissing {
                url: url.to_owned(),
            })
    }
}

/// Production synchronous HTTP implementation backed by `ureq`.
#[derive(Clone, Debug)]
pub struct UreqHttpClient {
    agent: ureq::Agent,
}

impl UreqHttpClient {
    /// Creates a production HTTP client with a reusable connection agent.
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new().build(),
        }
    }
}

impl Default for UreqHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for UreqHttpClient {
    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, RuntimeError> {
        let mut request = self.agent.get(url);
        for (name, value) in headers {
            request = request.set(name, value);
        }

        let response = request.call().map_err(|error| match error {
            ureq::Error::Status(status, _) => RuntimeError::HttpStatus {
                url: url.to_owned(),
                status,
            },
            ureq::Error::Transport(_) => RuntimeError::HttpRequest {
                url: url.to_owned(),
            },
        })?;
        let status = response.status();
        let headers = response
            .headers_names()
            .into_iter()
            .filter_map(|name| response.header(&name).map(|value| (name, value.to_owned())))
            .collect();
        let body = response
            .into_string()
            .map_err(|_| RuntimeError::HttpResponse {
                url: url.to_owned(),
            })?;

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}
