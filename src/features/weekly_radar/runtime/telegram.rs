//! Telegram Bot API transport for the rendered Weekly Radar report.
//!
//! Credentials are loaded only from the reserved environment variables. Public
//! errors retain no URL, token, chat ID, response body, or provider message.

use std::fmt;
use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

use super::report::RenderedReport;
use crate::features::weekly_radar::infrastructure::telegram_publisher::{
    TelegramMessageId, TelegramPublisherAdapter, TelegramPublisherError, TelegramTransport,
    TelegramTransportError, TELEGRAM_BOT_TOKEN_ENV, TELEGRAM_CHAT_ID_ENV,
};
use crate::features::weekly_radar::interface::semantic_message_splitter::{
    SemanticMessageChunk, SemanticMessageSplit, SemanticMessageSplitter, SemanticSplitError,
    SemanticSplitLimits,
};

const TELEGRAM_API: &str = "https://api.telegram.org";
const MAX_TELEGRAM_RESPONSE_BYTES: usize = 64 * 1024;
const TELEGRAM_MAX_CHARACTERS: usize = 4_096;
const TELEGRAM_MAX_LINES: usize = 120;

fn page_header(page_number: usize, total_pages: usize) -> String {
    format!("{page_number}/{total_pages}\n")
}

fn number_page(page_number: usize, total_pages: usize, markdown: &str) -> String {
    format!("{}{markdown}", page_header(page_number, total_pages))
}

fn numbered_chunks(split: &SemanticMessageSplit) -> Vec<SemanticMessageChunk> {
    let total_pages = split.chunks().len();
    split
        .chunks()
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            chunk.with_markdown(number_page(index + 1, total_pages, chunk.markdown()))
        })
        .collect()
}

/// Public, secret-safe failures for report rendering and delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TelegramError {
    /// The bot token environment variable was absent or blank.
    MissingBotToken,
    /// The chat ID environment variable was absent or blank.
    MissingChatId,
    /// The report could not be split at a safe semantic boundary.
    InvalidReport,
    /// A report chunk remained undelivered after the configured attempts.
    DeliveryFailed {
        /// Zero-based chunk index.
        chunk_index: usize,
        /// Number of attempts made for the failed chunk.
        attempts: u32,
        /// Number of earlier chunks accepted by Telegram.
        delivered_chunks: usize,
        /// Message IDs accepted before the failed chunk.
        successful_message_ids: Vec<TelegramMessageId>,
        /// Attempts used for the accepted message IDs, in the same order.
        successful_attempts: Vec<u32>,
    },
}

impl fmt::Display for TelegramError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBotToken => {
                formatter.write_str("Telegram bot credentials are unavailable")
            }
            Self::MissingChatId => formatter.write_str("Telegram destination is unavailable"),
            Self::InvalidReport => formatter.write_str("rendered report cannot be delivered safely"),
            Self::DeliveryFailed {
                chunk_index,
                attempts,
                delivered_chunks,
                ..
            } => write!(
                formatter,
                "Telegram delivery failed for chunk {chunk_index} after {attempts} attempt(s); {delivered_chunks} earlier chunk(s) were accepted"
            ),
        }
    }
}

impl std::error::Error for TelegramError {}

impl TelegramError {
    /// Returns accepted message IDs without exposing any provider error text.
    pub fn successful_message_ids(&self) -> &[TelegramMessageId] {
        match self {
            Self::DeliveryFailed {
                successful_message_ids,
                ..
            } => successful_message_ids,
            _ => &[],
        }
    }

    /// Returns attempts corresponding to [`Self::successful_message_ids`].
    pub fn successful_attempts(&self) -> &[u32] {
        match self {
            Self::DeliveryFailed {
                successful_attempts,
                ..
            } => successful_attempts,
            _ => &[],
        }
    }
}

/// Fixed retry policy for one rendered report delivery.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TelegramRetryPolicy {
    max_attempts: u32,
    delay: Duration,
}

impl TelegramRetryPolicy {
    /// Creates a policy; zero attempts is normalized to one safe attempt.
    pub const fn new(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts: if max_attempts == 0 { 1 } else { max_attempts },
            delay,
        }
    }

    /// Returns the default bounded retry policy.
    pub const fn default() -> Self {
        Self::new(3, Duration::from_millis(100))
    }

    /// Returns the configured maximum attempts.
    pub const fn max_attempts(self) -> u32 {
        self.max_attempts
    }

    /// Returns the delay between attempts.
    pub const fn delay(self) -> Duration {
        self.delay
    }
}

impl Default for TelegramRetryPolicy {
    fn default() -> Self {
        Self::new(3, Duration::from_millis(100))
    }
}

/// Ordered provider IDs returned for one rendered report delivery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelegramDeliveryReceipt {
    report_id: String,
    message_ids: Vec<TelegramMessageId>,
    attempts: Vec<u32>,
}

impl TelegramDeliveryReceipt {
    /// Returns the rendered report identity bound to this delivery.
    pub fn report_id(&self) -> &str {
        &self.report_id
    }

    /// Returns message IDs in exact rendered chunk order.
    pub fn message_ids(&self) -> &[TelegramMessageId] {
        &self.message_ids
    }

    /// Returns attempts used for each delivered chunk in the same order.
    pub fn attempts(&self) -> &[u32] {
        &self.attempts
    }

    /// Returns the number of delivered chunks.
    pub const fn chunk_count(&self) -> usize {
        self.message_ids.len()
    }
}

/// Environment-backed Telegram Bot API transport.
pub struct EnvTelegramTransport {
    token: String,
    chat_id: String,
}

impl fmt::Debug for EnvTelegramTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EnvTelegramTransport")
            .field("chat_id", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl EnvTelegramTransport {
    /// Loads credentials only from `ORGX_TELEGRAM_BOT_TOKEN` and
    /// `ORGX_TELEGRAM_CHAT_ID`.
    pub fn from_env() -> Result<Self, TelegramError> {
        let token = std::env::var(TELEGRAM_BOT_TOKEN_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or(TelegramError::MissingBotToken)?;
        let chat_id = std::env::var(TELEGRAM_CHAT_ID_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or(TelegramError::MissingChatId)?;
        Ok(Self { token, chat_id })
    }

    /// Returns the configured destination without exposing the token.
    pub fn chat_id(&self) -> &str {
        &self.chat_id
    }

    fn endpoint(&self) -> String {
        format!("{TELEGRAM_API}/bot{}/sendMessage", self.token)
    }
}

#[derive(Debug, Deserialize)]
struct TelegramApiResponse {
    ok: bool,
    result: Option<TelegramApiResult>,
}

#[derive(Debug, Deserialize)]
struct TelegramApiResult {
    message_id: i64,
}

fn safe_transport_error(kind: TelegramTransportError) -> TelegramTransportError {
    match kind {
        TelegramTransportError::Unavailable { .. } => TelegramTransportError::Unavailable {
            reason: "Telegram transport unavailable".to_owned(),
        },
        TelegramTransportError::Rejected { .. } => TelegramTransportError::Rejected {
            reason: "Telegram request rejected".to_owned(),
        },
        TelegramTransportError::Failed { .. } => TelegramTransportError::Failed {
            reason: "Telegram transport failed".to_owned(),
        },
    }
}

impl TelegramTransport for EnvTelegramTransport {
    fn send_message(
        &self,
        destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        if destination != self.chat_id {
            return Err(TelegramTransportError::Rejected {
                reason: "Telegram destination rejected".to_owned(),
            });
        }
        let body = serde_json::json!({
            "chat_id": destination,
            "text": markdown,
            "disable_web_page_preview": true,
        });
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(10))
            .timeout_write(Duration::from_secs(10))
            .redirects(0)
            .build();
        let response = match agent
            .post(&self.endpoint())
            .set("Content-Type", "application/json")
            .send_string(&body.to_string())
        {
            Ok(response) => response,
            Err(error) => {
                return Err(safe_transport_error(match error {
                    ureq::Error::Status(_, _) => TelegramTransportError::Rejected {
                        reason: String::new(),
                    },
                    ureq::Error::Transport(_) => TelegramTransportError::Unavailable {
                        reason: String::new(),
                    },
                }))
            }
        };

        let mut bytes = Vec::with_capacity(MAX_TELEGRAM_RESPONSE_BYTES + 1);
        response
            .into_reader()
            .take((MAX_TELEGRAM_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| {
                safe_transport_error(TelegramTransportError::Failed {
                    reason: String::new(),
                })
            })?;
        if bytes.len() > MAX_TELEGRAM_RESPONSE_BYTES {
            return Err(TelegramTransportError::Failed {
                reason: "Telegram response exceeded configured limit".to_owned(),
            });
        }
        let parsed: TelegramApiResponse = serde_json::from_slice(&bytes).map_err(|_| {
            safe_transport_error(TelegramTransportError::Failed {
                reason: String::new(),
            })
        })?;
        if !parsed.ok {
            return Err(TelegramTransportError::Rejected {
                reason: "Telegram request rejected".to_owned(),
            });
        }
        let message_id = parsed.result.ok_or(TelegramTransportError::Rejected {
            reason: "Telegram response missing message ID".to_owned(),
        })?;
        TelegramMessageId::new(message_id.message_id.to_string()).map_err(|_| {
            TelegramTransportError::Rejected {
                reason: "Telegram response contained an invalid message ID".to_owned(),
            }
        })
    }
}

struct BorrowedTransport<'a, T: ?Sized>(&'a T);

impl<T: TelegramTransport + ?Sized> TelegramTransport for BorrowedTransport<'_, T> {
    fn send_message(
        &self,
        destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        self.0.send_message(destination, markdown)
    }
}

fn map_split_error(_error: SemanticSplitError) -> TelegramError {
    TelegramError::InvalidReport
}

fn map_publisher_error(
    error: TelegramPublisherError,
    chunk_index: usize,
    attempts: u32,
    delivered_chunks: usize,
    prior_message_ids: &[TelegramMessageId],
    prior_attempts: &[u32],
) -> Result<TelegramDeliveryReceipt, TelegramError> {
    let publisher_message_ids = match error {
        TelegramPublisherError::Transport {
            successful_message_ids,
            ..
        } => successful_message_ids,
        _ => Vec::new(),
    };
    let mut successful_message_ids = prior_message_ids.to_vec();
    successful_message_ids.extend(publisher_message_ids.iter().cloned());
    let mut successful_attempts = prior_attempts.to_vec();
    successful_attempts.extend(std::iter::repeat_n(attempts, publisher_message_ids.len()));
    Err(TelegramError::DeliveryFailed {
        chunk_index,
        attempts,
        delivered_chunks,
        successful_message_ids,
        successful_attempts,
    })
}

fn split_for_telegram_delivery(
    report: &RenderedReport,
) -> Result<Vec<SemanticMessageChunk>, TelegramError> {
    let max_lines = TELEGRAM_MAX_LINES
        .checked_sub(1)
        .ok_or(TelegramError::InvalidReport)?;
    let mut max_characters = TELEGRAM_MAX_CHARACTERS;

    loop {
        let limits =
            SemanticSplitLimits::new(max_characters, max_lines).map_err(map_split_error)?;
        let split =
            SemanticMessageSplitter::split(report.markdown(), limits).map_err(map_split_error)?;
        if split.is_empty() {
            return Err(TelegramError::InvalidReport);
        }

        let total_pages = split.chunks().len();
        let page_header_characters = page_header(total_pages, total_pages).chars().count();
        let required_max_characters = TELEGRAM_MAX_CHARACTERS
            .checked_sub(page_header_characters)
            .ok_or(TelegramError::InvalidReport)?;
        if max_characters != required_max_characters {
            max_characters = required_max_characters;
            continue;
        }

        return Ok(numbered_chunks(&split));
    }
}

/// Splits and delivers a rendered report using credentials from the environment.
pub fn send_rendered_report(
    report: &RenderedReport,
) -> Result<TelegramDeliveryReceipt, TelegramError> {
    let transport = EnvTelegramTransport::from_env()?;
    let destination = transport.chat_id().to_owned();
    send_rendered_report_with_transport(
        report,
        &destination,
        &transport,
        TelegramRetryPolicy::default(),
    )
}

/// Splits and delivers a report through an injected transport, preserving exact
/// chunk order and retrying each chunk without resending earlier successes.
pub fn send_rendered_report_with_transport<T: TelegramTransport + ?Sized>(
    report: &RenderedReport,
    destination: &str,
    transport: &T,
    policy: TelegramRetryPolicy,
) -> Result<TelegramDeliveryReceipt, TelegramError> {
    let numbered_chunks = split_for_telegram_delivery(report)?;
    let publisher = TelegramPublisherAdapter::new(destination, BorrowedTransport(transport))
        .map_err(|_| TelegramError::InvalidReport)?;
    let mut message_ids = Vec::with_capacity(numbered_chunks.len());
    let mut attempts_used = Vec::with_capacity(numbered_chunks.len());

    for (chunk_index, chunk) in numbered_chunks.iter().enumerate() {
        let mut delivered = None;
        for attempt in 1..=policy.max_attempts() {
            match publisher.publish_chunks_with_ids(std::slice::from_ref(chunk)) {
                Ok(mut ids) => {
                    delivered = ids.pop();
                    attempts_used.push(attempt);
                    break;
                }
                Err(error) if attempt < policy.max_attempts() => {
                    let _ = safe_transport_error(TelegramTransportError::Failed {
                        reason: error.to_string(),
                    });
                    if !policy.delay.is_zero() {
                        std::thread::sleep(policy.delay);
                    }
                }
                Err(error) => {
                    return map_publisher_error(
                        error,
                        chunk_index,
                        attempt,
                        message_ids.len(),
                        &message_ids,
                        &attempts_used,
                    )
                }
            }
        }
        if let Some(message_id) = delivered {
            message_ids.push(message_id);
        } else {
            return Err(TelegramError::DeliveryFailed {
                chunk_index,
                attempts: policy.max_attempts(),
                delivered_chunks: message_ids.len(),
                successful_message_ids: message_ids,
                successful_attempts: attempts_used,
            });
        }
    }

    Ok(TelegramDeliveryReceipt {
        report_id: report.report_id().to_owned(),
        message_ids,
        attempts: attempts_used,
    })
}

#[cfg(test)]
mod page_numbering_tests {
    use super::{numbered_chunks, SemanticMessageSplitter, SemanticSplitLimits};

    #[test]
    fn prefixes_each_of_eight_semantic_pages_with_one_based_number_and_total() {
        let rendered = concat!(
            "## AI 时代范本验证\n",
            "### Acme\n- one\n- two\n- three\n",
            "### Beta\n- one\n- two\n- three\n",
            "### Gamma\n- one\n- two\n- three\n",
            "### Delta\n- one\n- two\n- three\n",
            "### Epsilon\n- one\n- two\n- three\n",
            "### Zeta\n- one\n- two\n- three\n",
            "### Eta\n- one\n- two\n- three\n",
            "### Theta\n- one\n- two\n- three\n",
        );
        let split = SemanticMessageSplitter::split(
            rendered,
            SemanticSplitLimits::new(4_096, 5).expect("test limits should be valid"),
        )
        .expect("eight complete entries should become eight pages");
        assert_eq!(split.chunks().len(), 8);

        let numbered = numbered_chunks(&split);
        let headers = numbered
            .iter()
            .map(|chunk| chunk.markdown().lines().next().expect("page has a header"))
            .collect::<Vec<_>>();
        assert_eq!(
            headers,
            ["1/8", "2/8", "3/8", "4/8", "5/8", "6/8", "7/8", "8/8"]
        );

        let unnumbered = numbered
            .iter()
            .map(|chunk| {
                chunk
                    .markdown()
                    .split_once('\n')
                    .expect("page should contain a header newline")
                    .1
            })
            .collect::<String>();
        assert_eq!(unnumbered, rendered);
    }
}
