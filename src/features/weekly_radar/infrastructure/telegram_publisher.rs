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

/// Provider-issued identifier for one successfully accepted Telegram message.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TelegramMessageId(String);

impl TelegramMessageId {
    /// Creates an ID and rejects blank provider output.
    pub fn new(value: impl Into<String>) -> Result<Self, TelegramPublisherError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(TelegramPublisherError::BlankMessageId);
        }
        Ok(Self(value))
    }

    /// Returns the exact provider-issued identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
    /// Sends one complete message, returns its provider ID, and reports a transport failure.
    fn send_message(
        &self,
        destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError>;
}

/// Typed failures raised before or during ordered chunk delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TelegramPublisherError {
    /// The configured destination was blank.
    BlankDestination,
    /// The caller supplied no complete chunks to deliver.
    EmptySplit,
    /// The provider returned a blank message ID.
    BlankMessageId,
    /// A transport failed for one specific source-ordered chunk.
    Transport {
        /// Zero-based source chunk index.
        chunk_index: usize,
        /// Semantic boundary of the failed chunk.
        boundary: SemanticBoundary,
        /// IDs accepted before the failing message.
        successful_message_ids: Vec<TelegramMessageId>,
        /// Provider-neutral transport explanation.
        reason: String,
    },
}

impl fmt::Display for TelegramPublisherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlankDestination => formatter.write_str("Telegram destination cannot be blank"),
            Self::EmptySplit => formatter.write_str("Telegram message split cannot be empty"),
            Self::BlankMessageId => formatter.write_str("Telegram message ID cannot be blank"),
            Self::Transport {
                chunk_index,
                boundary,
                reason,
                ..
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
        self.publish_split_with_ids(message_split).map(|_| ())
    }

    /// Forwards complete WR-011 chunks and returns provider IDs in source order.
    pub fn publish_split_with_ids(
        &self,
        message_split: &SemanticMessageSplit,
    ) -> Result<Vec<TelegramMessageId>, TelegramPublisherError> {
        self.publish_chunks_with_ids(message_split.chunks())
    }

    /// Forwards a caller-supplied ordered chunk collection.
    pub fn publish_chunks(
        &self,
        chunks: &[SemanticMessageChunk],
    ) -> Result<(), TelegramPublisherError> {
        self.publish_chunks_with_ids(chunks).map(|_| ())
    }

    /// Forwards chunks and returns all successful provider IDs in source order.
    pub fn publish_chunks_with_ids(
        &self,
        chunks: &[SemanticMessageChunk],
    ) -> Result<Vec<TelegramMessageId>, TelegramPublisherError> {
        if chunks.is_empty() {
            return Err(TelegramPublisherError::EmptySplit);
        }

        let mut message_ids = Vec::with_capacity(chunks.len());
        for (chunk_index, chunk) in chunks.iter().enumerate() {
            match self
                .transport
                .send_message(&self.destination, chunk.markdown())
            {
                Ok(message_id) => message_ids.push(message_id),
                Err(error) => {
                    return Err(TelegramPublisherError::Transport {
                        chunk_index,
                        boundary: chunk.boundary(),
                        successful_message_ids: message_ids,
                        reason: error.to_string(),
                    });
                }
            }
        }
        Ok(message_ids)
    }

    /// Delivers opaque precomputed publication facts and returns provider IDs.
    pub fn publish_publication_with_ids(
        &self,
        publication: &WeeklyRadarPublication,
    ) -> Result<Vec<TelegramMessageId>, TelegramPublisherError> {
        if publication.facts().is_empty() {
            return Err(TelegramPublisherError::EmptySplit);
        }

        let mut message_ids = Vec::with_capacity(publication.facts().len());
        for (message_index, fact) in publication.facts().iter().enumerate() {
            match self
                .transport
                .send_message(&self.destination, fact.value().as_str())
            {
                Ok(message_id) => message_ids.push(message_id),
                Err(error) => {
                    return Err(TelegramPublisherError::Transport {
                        chunk_index: message_index,
                        boundary: SemanticBoundary::ExecutiveSummary,
                        successful_message_ids: message_ids,
                        reason: error.to_string(),
                    });
                }
            }
        }
        Ok(message_ids)
    }
}

impl<T: TelegramTransport> WeeklyRadarPublisher for TelegramWeeklyRadarPublisher<T> {
    /// Delivers opaque precomputed publication facts in their supplied order.
    ///
    /// The application port predates WR-011's typed split. Its facts are
    /// treated as already-rendered message payloads here; callers that have a
    /// semantic split should use [`Self::publish_split`] directly.
    fn publish(&self, publication: &WeeklyRadarPublication) -> Result<(), WeeklyRadarPublishError> {
        match self.publish_publication_with_ids(publication) {
            Ok(_) => Ok(()),
            Err(error) => {
                let reason = error.to_string();
                Err(match error {
                    TelegramPublisherError::EmptySplit => WeeklyRadarPublishError::Rejected {
                        reason: "publication contains no precomputed messages".to_owned(),
                    },
                    TelegramPublisherError::Transport { .. } => {
                        WeeklyRadarPublishError::Failed { reason }
                    }
                    TelegramPublisherError::BlankDestination
                    | TelegramPublisherError::BlankMessageId => {
                        WeeklyRadarPublishError::Rejected { reason }
                    }
                })
            }
        }
    }
}

#[cfg(test)]
#[path = "telegram_publisher_test.rs"]
mod module_tests;
