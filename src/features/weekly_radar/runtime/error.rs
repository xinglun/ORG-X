//! Typed failures shared by Weekly Radar runtime boundaries.

use std::fmt;

/// Error returned by runtime configuration, model, and transport boundaries.
///
/// Variants retain operation context without retaining request headers, bodies,
/// credentials, or other secret-bearing values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    /// A registry or runtime setting failed validation.
    InvalidConfiguration { reason: String },
    /// A normalized runtime model failed validation.
    InvalidModel { reason: String },
    /// A registry or other runtime JSON document could not be decoded.
    JsonDecode { context: String },
    /// A local configuration file could not be read.
    ConfigurationIo { path: String },
    /// An HTTP request failed before receiving a usable response.
    HttpRequest { url: String },
    /// An HTTP endpoint returned a non-success status.
    HttpStatus { url: String, status: u16 },
    /// An HTTP response body could not be read.
    HttpResponse { url: String },
    /// A fixture transport has no response for the requested URL.
    FixtureMissing { url: String },
    /// A shared fixture transport could not be accessed.
    FixtureState,
}

impl RuntimeError {
    pub(crate) fn invalid_configuration(reason: impl Into<String>) -> Self {
        Self::InvalidConfiguration {
            reason: reason.into(),
        }
    }

    pub(crate) fn invalid_model(reason: impl Into<String>) -> Self {
        Self::InvalidModel {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration { reason } => {
                write!(formatter, "invalid runtime configuration: {reason}")
            }
            Self::InvalidModel { reason } => write!(formatter, "invalid runtime model: {reason}"),
            Self::JsonDecode { context } => write!(formatter, "invalid JSON in {context}"),
            Self::ConfigurationIo { path } => {
                write!(formatter, "could not read runtime configuration at {path}")
            }
            Self::HttpRequest { url } => write!(formatter, "HTTP request failed for {url}"),
            Self::HttpStatus { url, status } => {
                write!(formatter, "HTTP request for {url} returned status {status}")
            }
            Self::HttpResponse { url } => {
                write!(formatter, "HTTP response body could not be read for {url}")
            }
            Self::FixtureMissing { url } => {
                write!(formatter, "fixture response is missing for {url}")
            }
            Self::FixtureState => formatter.write_str("fixture transport state is unavailable"),
        }
    }
}

impl std::error::Error for RuntimeError {}
