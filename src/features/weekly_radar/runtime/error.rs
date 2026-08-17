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
    HttpRequest,
    /// An HTTP response body could not be read.
    HttpResponse,
    /// An HTTP response exceeded the finite runtime body limit.
    HttpResponseTooLarge,
    /// A fixture transport has no response for the requested URL.
    FixtureMissing,
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
            Self::HttpRequest => formatter.write_str("HTTP request failed"),
            Self::HttpResponse => formatter.write_str("HTTP response body could not be read"),
            Self::HttpResponseTooLarge => {
                formatter.write_str("HTTP response body exceeded configured limit")
            }
            Self::FixtureMissing => formatter.write_str("fixture response is missing"),
            Self::FixtureState => formatter.write_str("fixture transport state is unavailable"),
        }
    }
}

impl std::error::Error for RuntimeError {}
