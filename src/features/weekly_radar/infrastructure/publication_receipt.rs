//! Immutable publication receipts and explicit delivery-only retry.
//!
//! This boundary retains one precomputed [`WeeklyRadarPublication`] and sends
//! its opaque message values through a receipt-aware publisher. It never
//! recalculates a report, creates a snapshot, or contacts Telegram directly.

use std::fmt;

use crate::features::weekly_radar::{
    domain::{SnapshotId, WeeklyRadarPublication},
    infrastructure::telegram_publisher::{
        TelegramMessageId, TelegramPublisherError, TelegramTransport, TelegramWeeklyRadarPublisher,
    },
};

/// Publication channel represented by this receipt boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationChannel {
    /// Telegram delivery.
    Telegram,
}

/// Delivery status recorded for one publication attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationStatus {
    /// Every precomputed message was accepted by the transport.
    Published,
    /// At least one message was accepted before a later message failed.
    Partial { failed_message_index: usize },
    /// The first message failed, so no message ID was accepted.
    Failed { failed_message_index: usize },
}

/// Validation failures for a manually constructed receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationReceiptError {
    /// The supplied timestamp was blank.
    BlankPublishedAt,
    /// Attempt numbers start at one.
    InvalidAttempt,
    /// A Published receipt must retain at least one provider message ID.
    PublishedWithoutMessageIds,
}

impl fmt::Display for PublicationReceiptError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlankPublishedAt => formatter.write_str("published timestamp cannot be blank"),
            Self::InvalidAttempt => {
                formatter.write_str("publication attempt must be greater than zero")
            }
            Self::PublishedWithoutMessageIds => {
                formatter.write_str("published receipt must contain message IDs")
            }
        }
    }
}

impl std::error::Error for PublicationReceiptError {}

/// Immutable evidence of one delivery attempt for one Weekly Radar snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationReceipt {
    channel: PublicationChannel,
    snapshot_id: SnapshotId,
    published_at: String,
    message_ids: Vec<TelegramMessageId>,
    status: PublicationStatus,
    attempt: u32,
}

impl PublicationReceipt {
    /// Creates a validated receipt without reading a clock or provider.
    pub fn new(
        channel: PublicationChannel,
        snapshot_id: SnapshotId,
        published_at: impl Into<String>,
        message_ids: Vec<TelegramMessageId>,
        status: PublicationStatus,
        attempt: u32,
    ) -> Result<Self, PublicationReceiptError> {
        let published_at = published_at.into();
        if published_at.trim().is_empty() {
            return Err(PublicationReceiptError::BlankPublishedAt);
        }
        if attempt == 0 {
            return Err(PublicationReceiptError::InvalidAttempt);
        }
        if status == PublicationStatus::Published && message_ids.is_empty() {
            return Err(PublicationReceiptError::PublishedWithoutMessageIds);
        }
        Ok(Self {
            channel,
            snapshot_id,
            published_at,
            message_ids,
            status,
            attempt,
        })
    }

    /// Returns the publication channel.
    pub const fn channel(&self) -> PublicationChannel {
        self.channel
    }

    /// Returns the immutable snapshot identity.
    pub fn snapshot_id(&self) -> &SnapshotId {
        &self.snapshot_id
    }

    /// Returns the supplied publication timestamp.
    pub fn published_at(&self) -> &str {
        &self.published_at
    }

    /// Returns provider message IDs in source order.
    pub fn message_ids(&self) -> &[TelegramMessageId] {
        &self.message_ids
    }

    /// Returns the typed delivery status.
    pub fn status(&self) -> &PublicationStatus {
        &self.status
    }

    /// Returns the one-based attempt number.
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }
}

/// Failure returned with the receipt produced by the failed attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationDeliveryFailure {
    receipt: PublicationReceipt,
    error: Box<PublicationDeliveryError>,
}

impl PublicationDeliveryFailure {
    fn new(receipt: PublicationReceipt, error: PublicationDeliveryError) -> Self {
        Self {
            receipt,
            error: Box::new(error),
        }
    }

    /// Returns the receipt that proves what the failed attempt accepted.
    pub fn receipt(&self) -> &PublicationReceipt {
        &self.receipt
    }

    /// Returns the typed delivery failure.
    pub fn error(&self) -> &PublicationDeliveryError {
        &self.error
    }
}

/// Deterministic delivery or retry validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublicationDeliveryError {
    /// The retained publication contains no precomputed messages.
    NoMessages,
    /// The retry receipt belongs to another immutable snapshot.
    SnapshotMismatch { expected: String, actual: String },
    /// A Published receipt cannot be retried by this explicit retry boundary.
    AlreadyPublished,
    /// The provider failed for one source-ordered message.
    Transport {
        message_index: usize,
        reason: String,
    },
}

impl fmt::Display for PublicationDeliveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMessages => formatter.write_str("publication contains no messages"),
            Self::SnapshotMismatch { expected, actual } => {
                write!(
                    formatter,
                    "retry snapshot mismatch: expected {expected}, received {actual}"
                )
            }
            Self::AlreadyPublished => formatter.write_str("published receipt cannot be retried"),
            Self::Transport {
                message_index,
                reason,
            } => write!(
                formatter,
                "delivery failed for message {message_index}: {reason}"
            ),
        }
    }
}

impl std::error::Error for PublicationDeliveryError {}

/// Clock boundary used to timestamp receipts without coupling this module to a runtime.
pub trait PublicationClock {
    /// Returns an already-formatted timestamp for the current attempt.
    fn now(&self) -> &str;
}

/// Port for publishing an immutable publication and returning provider IDs.
pub trait ReceiptAwarePublisher {
    /// Delivers the supplied precomputed publication without recalculation.
    fn publish_publication_with_ids(
        &self,
        publication: &WeeklyRadarPublication,
    ) -> Result<Vec<TelegramMessageId>, TelegramPublisherError>;
}

impl<T: TelegramTransport> ReceiptAwarePublisher for TelegramWeeklyRadarPublisher<T> {
    fn publish_publication_with_ids(
        &self,
        publication: &WeeklyRadarPublication,
    ) -> Result<Vec<TelegramMessageId>, TelegramPublisherError> {
        TelegramWeeklyRadarPublisher::publish_publication_with_ids(self, publication)
    }
}

/// Explicit delivery service retaining one immutable publication for retry.
pub struct PublicationReceiptService<P, C> {
    publication: WeeklyRadarPublication,
    publisher: P,
    clock: C,
}

impl<P, C> PublicationReceiptService<P, C>
where
    P: ReceiptAwarePublisher,
    C: PublicationClock,
{
    /// Creates a delivery service around one retained publication value.
    pub fn new(publication: WeeklyRadarPublication, publisher: P, clock: C) -> Self {
        Self {
            publication,
            publisher,
            clock,
        }
    }

    /// Delivers the retained publication as attempt one.
    pub fn publish(&self) -> Result<PublicationReceipt, PublicationDeliveryFailure> {
        self.deliver(1)
    }

    /// Retries delivery only when the receipt belongs to this same snapshot.
    pub fn retry(
        &self,
        previous: &PublicationReceipt,
    ) -> Result<PublicationReceipt, PublicationDeliveryFailure> {
        if previous.snapshot_id() != self.publication.snapshot_id() {
            return Err(PublicationDeliveryFailure::new(
                previous.clone(),
                PublicationDeliveryError::SnapshotMismatch {
                    expected: self.publication.snapshot_id().as_str().to_owned(),
                    actual: previous.snapshot_id().as_str().to_owned(),
                },
            ));
        }
        if previous.status() == &PublicationStatus::Published {
            return Err(PublicationDeliveryFailure::new(
                previous.clone(),
                PublicationDeliveryError::AlreadyPublished,
            ));
        }
        self.deliver(previous.attempt().saturating_add(1))
    }

    fn deliver(&self, attempt: u32) -> Result<PublicationReceipt, PublicationDeliveryFailure> {
        if self.publication.facts().is_empty() {
            return Err(PublicationDeliveryFailure::new(
                self.receipt(
                    PublicationStatus::Failed {
                        failed_message_index: 0,
                    },
                    Vec::new(),
                    attempt,
                ),
                PublicationDeliveryError::NoMessages,
            ));
        }

        match self
            .publisher
            .publish_publication_with_ids(&self.publication)
        {
            Ok(message_ids) => Ok(self.receipt(PublicationStatus::Published, message_ids, attempt)),
            Err(error) => {
                let reason = error.to_string();
                let (message_index, successful_message_ids) = match error {
                    TelegramPublisherError::Transport {
                        chunk_index,
                        successful_message_ids,
                        ..
                    } => (chunk_index, successful_message_ids),
                    TelegramPublisherError::EmptySplit => (0, Vec::new()),
                    TelegramPublisherError::BlankDestination
                    | TelegramPublisherError::BlankMessageId => (0, Vec::new()),
                };
                let status = if successful_message_ids.is_empty() {
                    PublicationStatus::Failed {
                        failed_message_index: message_index,
                    }
                } else {
                    PublicationStatus::Partial {
                        failed_message_index: message_index,
                    }
                };
                let receipt = self.receipt(status, successful_message_ids, attempt);
                Err(PublicationDeliveryFailure::new(
                    receipt,
                    PublicationDeliveryError::Transport {
                        message_index,
                        reason,
                    },
                ))
            }
        }
    }

    fn receipt(
        &self,
        status: PublicationStatus,
        message_ids: Vec<TelegramMessageId>,
        attempt: u32,
    ) -> PublicationReceipt {
        PublicationReceipt::new(
            PublicationChannel::Telegram,
            self.publication.snapshot_id().clone(),
            self.clock.now(),
            message_ids,
            status,
            attempt,
        )
        .expect("receipt service must receive a nonblank clock and valid attempt")
    }
}

#[cfg(test)]
#[path = "publication_receipt_test.rs"]
mod module_tests;
