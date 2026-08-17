//! Provider-agnostic delivery boundary for complete Weekly Radar Telegram chunks.
//!
//! This module deliberately stops at an injected transport. It does not access
//! Telegram, read secrets, calculate domain values, create receipts, or retry a
//! failed send. A concrete provider transport can be added behind this boundary
//! in a later Work Item.

use std::fmt;

use crate::features::weekly_radar::{
    application::{WeeklyRadarPublishError, WeeklyRadarPublisher},
    domain::WeeklyRadarPublication,
    interface::semantic_message_splitter::{
        SemanticBoundary, SemanticMessageChunk, SemanticMessageSplit,
    },
};

/// Reserved environment variable name for a later Telegram bot token source.
pub const TELEGRAM_BOT_TOKEN_ENV: &str = "ORGX_TELEGRAM_BOT_TOKEN";

/// Reserved environment variable name for a later Telegram chat destination.
pub const TELEGRAM_CHAT_ID_ENV: &str = "ORGX_TELEGRAM_CHAT_ID";

/// Failure reported by an injected delivery transport.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TelegramTransportError {
    /// The transport is not available for this send attempt.
    Unavailable { reason: String },
    /// The transport rejected the destination or payload.
    Rejected { reason: String },
    /// The transport failed without changing the supplied payload.
    Failed { reason: String },
}

impl fmt::Display for TelegramTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { reason } | Self::Rejected { reason } | Self::Failed { reason } => {
                formatter.write_str(reason)
            }
        }
    }
}

impl std::error::Error for TelegramTransportError {}

/// Provider-agnostic port for sending one complete Markdown message.
///
/// Implementations own the provider client and any runtime credentials. The
/// adapter passes the destination and exact Markdown only; this trait does not
/// prescribe HTTP, a Telegram SDK, secret storage, or retry behavior.
pub trait TelegramTransport {
    /// Sends one complete message and reports a transport boundary failure.
    fn send_message(&self, destination: &str, markdown: &str)
        -> Result<(), TelegramTransportError>;
}

/// Typed failures raised before or during ordered chunk delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TelegramPublisherError {
    /// The configured destination was blank.
    BlankDestination,
    /// The caller supplied no complete chunks to deliver.
    EmptySplit,
    /// A transport failed for one specific source-ordered chunk.
    Transport {
        /// Zero-based source chunk index.
        chunk_index: usize,
        /// Semantic boundary of the failed chunk.
        boundary: SemanticBoundary,
        /// Provider-neutral transport explanation.
        reason: String,
    },
}

impl fmt::Display for TelegramPublisherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlankDestination => formatter.write_str("Telegram destination cannot be blank"),
            Self::EmptySplit => formatter.write_str("Telegram message split cannot be empty"),
            Self::Transport {
                chunk_index,
                boundary,
                reason,
            } => write!(
                formatter,
                "Telegram transport failed for chunk {chunk_index} ({}): {reason}",
                boundary.as_str()
            ),
        }
    }
}

impl std::error::Error for TelegramPublisherError {}

/// Infrastructure adapter that sends complete chunks through an injected port.
pub struct TelegramWeeklyRadarPublisher<T> {
    destination: String,
    transport: T,
}

/// Compatibility name for the Telegram infrastructure adapter.
pub type TelegramPublisherAdapter<T> = TelegramWeeklyRadarPublisher<T>;

impl<T> TelegramWeeklyRadarPublisher<T> {
    /// Creates an adapter and rejects a blank destination before any send.
    pub fn new(
        destination: impl Into<String>,
        transport: T,
    ) -> Result<Self, TelegramPublisherError> {
        let destination = destination.into();
        if destination.trim().is_empty() {
            return Err(TelegramPublisherError::BlankDestination);
        }
        Ok(Self {
            destination,
            transport,
        })
    }

    /// Returns the destination supplied to the adapter.
    pub fn destination(&self) -> &str {
        &self.destination
    }
}

impl<T: TelegramTransport> TelegramWeeklyRadarPublisher<T> {
    /// Forwards complete WR-011 chunks exactly once and in source order.
    pub fn publish_split(
        &self,
        message_split: &SemanticMessageSplit,
    ) -> Result<(), TelegramPublisherError> {
        self.publish_chunks(message_split.chunks())
    }

    /// Forwards a caller-supplied ordered chunk collection.
    pub fn publish_chunks(
        &self,
        chunks: &[SemanticMessageChunk],
    ) -> Result<(), TelegramPublisherError> {
        if chunks.is_empty() {
            return Err(TelegramPublisherError::EmptySplit);
        }

        for (chunk_index, chunk) in chunks.iter().enumerate() {
            if let Err(error) = self
                .transport
                .send_message(&self.destination, chunk.markdown())
            {
                return Err(TelegramPublisherError::Transport {
                    chunk_index,
                    boundary: chunk.boundary(),
                    reason: error.to_string(),
                });
            }
        }
        Ok(())
    }
}

impl<T: TelegramTransport> WeeklyRadarPublisher for TelegramWeeklyRadarPublisher<T> {
    /// Delivers opaque precomputed publication facts in their supplied order.
    ///
    /// The application port predates WR-011's typed split. Its facts are
    /// treated as already-rendered message payloads here; callers that have a
    /// semantic split should use [`Self::publish_split`] directly.
    fn publish(&self, publication: &WeeklyRadarPublication) -> Result<(), WeeklyRadarPublishError> {
        if publication.facts().is_empty() {
            return Err(WeeklyRadarPublishError::Rejected {
                reason: "publication contains no precomputed messages".to_owned(),
            });
        }

        for (index, fact) in publication.facts().iter().enumerate() {
            if let Err(error) = self
                .transport
                .send_message(&self.destination, fact.value().as_str())
            {
                let reason = format!("publication message {index}: {error}");
                return Err(match error {
                    TelegramTransportError::Unavailable { .. } => {
                        WeeklyRadarPublishError::Unavailable
                    }
                    TelegramTransportError::Rejected { .. } => {
                        WeeklyRadarPublishError::Rejected { reason }
                    }
                    TelegramTransportError::Failed { .. } => {
                        WeeklyRadarPublishError::Failed { reason }
                    }
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "telegram_publisher_test.rs"]
mod module_tests;
