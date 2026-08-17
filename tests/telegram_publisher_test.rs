use std::sync::{Arc, Mutex};

use org_x::features::weekly_radar::infrastructure::telegram_publisher::{
    TelegramMessageId, TelegramPublisherAdapter, TelegramTransport, TelegramTransportError,
};
use org_x::features::weekly_radar::interface::semantic_message_splitter::{
    SemanticMessageSplitter, SemanticSplitLimits,
};

#[derive(Clone, Default)]
struct RecordingTransport(Arc<Mutex<Vec<String>>>);

impl TelegramTransport for RecordingTransport {
    fn send_message(
        &self,
        _destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        self.0
            .lock()
            .expect("recording lock should not fail")
            .push(markdown.to_owned());
        TelegramMessageId::new(format!("message-{}", self.0.lock().expect("lock").len())).map_err(
            |error| TelegramTransportError::Failed {
                reason: error.to_string(),
            },
        )
    }
}

#[test]
fn public_adapter_forwards_the_same_chunk_strings_in_order() {
    let transport = RecordingTransport::default();
    let adapter = TelegramPublisherAdapter::new("chat-123", transport.clone())
        .expect("destination should be accepted");
    let split = SemanticMessageSplitter::split(
        "# Weekly Radar\n\n## Executive Summary\nStable.\n\n## System Health\nHealthy.\n",
        SemanticSplitLimits::new(200, 20).expect("limits should be valid"),
    )
    .expect("fixture should split");

    adapter
        .publish_split(&split)
        .expect("delivery should succeed");

    let sent = transport.0.lock().expect("recording lock should not fail");
    let expected: Vec<_> = split
        .chunks()
        .iter()
        .map(|chunk| chunk.markdown())
        .collect();
    assert_eq!(
        sent.iter().map(String::as_str).collect::<Vec<_>>(),
        expected
    );
}
