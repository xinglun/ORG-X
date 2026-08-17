use std::sync::{Arc, Mutex};

use crate::features::weekly_radar::interface::semantic_message_splitter::{
    SemanticBoundary, SemanticMessageSplit, SemanticMessageSplitter, SemanticSplitLimits,
};

use super::{
    TelegramMessageId, TelegramPublisherAdapter, TelegramPublisherError, TelegramTransport,
    TelegramTransportError,
};

#[derive(Clone, Default)]
struct RecordingTransport {
    calls: Arc<Mutex<Vec<(String, String)>>>,
    fail_at: Option<usize>,
}

impl RecordingTransport {
    fn with_failure(fail_at: usize) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            fail_at: Some(fail_at),
        }
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls
            .lock()
            .expect("recording lock should not fail")
            .clone()
    }
}

impl TelegramTransport for RecordingTransport {
    fn send_message(
        &self,
        destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        let mut calls = self.calls.lock().expect("recording lock should not fail");
        let index = calls.len();
        calls.push((destination.to_owned(), markdown.to_owned()));
        if self.fail_at == Some(index) {
            return Err(TelegramTransportError::Failed {
                reason: "synthetic transport failure".to_owned(),
            });
        }
        TelegramMessageId::new(format!("message-{index}")).map_err(|error| {
            TelegramTransportError::Failed {
                reason: error.to_string(),
            }
        })
    }
}

fn split() -> SemanticMessageSplit {
    SemanticMessageSplitter::split(
        "# Weekly Radar\n\n## Executive Summary\nStable.\n\n## System Health\nHealthy.\n",
        SemanticSplitLimits::new(200, 20).expect("limits should be valid"),
    )
    .expect("fixture should split")
}

#[test]
fn forwards_complete_chunks_without_rewriting_or_reordering() {
    let transport = RecordingTransport::default();
    let adapter = TelegramPublisherAdapter::new("chat-123", transport.clone())
        .expect("destination should be accepted");
    let message_split = split();

    adapter
        .publish_split(&message_split)
        .expect("all chunks should be delivered");

    let calls = transport.calls();
    assert_eq!(calls.len(), message_split.chunks().len());
    for (call, chunk) in calls.iter().zip(message_split.chunks()) {
        assert_eq!(call.0, "chat-123");
        assert_eq!(call.1, chunk.markdown());
    }
}

#[test]
fn rejects_blank_destination_before_transport_invocation() {
    let transport = RecordingTransport::default();

    assert!(matches!(
        TelegramPublisherAdapter::new("  ", transport.clone()),
        Err(TelegramPublisherError::BlankDestination)
    ));
    assert!(transport.calls().is_empty());
}

#[test]
fn rejects_empty_split_before_transport_invocation() {
    let transport = RecordingTransport::default();
    let adapter = TelegramPublisherAdapter::new("chat-123", transport.clone())
        .expect("destination should be accepted");
    assert_eq!(
        adapter.publish_chunks(&[]),
        Err(TelegramPublisherError::EmptySplit)
    );
    assert!(transport.calls().is_empty());
}

#[test]
fn stops_after_first_transport_failure_and_reports_chunk_context() {
    let transport = RecordingTransport::with_failure(1);
    let adapter = TelegramPublisherAdapter::new("chat-123", transport.clone())
        .expect("destination should be accepted");

    let error = adapter
        .publish_split(&split())
        .expect_err("second chunk should fail");

    assert_eq!(
        error,
        TelegramPublisherError::Transport {
            chunk_index: 1,
            boundary: SemanticBoundary::SystemHealth,
            successful_message_ids: vec![TelegramMessageId::new("message-0").unwrap()],
            reason: "synthetic transport failure".to_owned(),
        }
    );
    assert_eq!(transport.calls().len(), 2);
}
