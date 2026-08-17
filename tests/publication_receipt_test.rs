use org_x::features::weekly_radar::domain::SnapshotId;
use org_x::features::weekly_radar::infrastructure::publication_receipt::{
    PublicationChannel, PublicationReceipt, PublicationStatus,
};
use org_x::features::weekly_radar::infrastructure::telegram_publisher::TelegramMessageId;

#[test]
fn receipt_public_fields_are_typed_and_stable() {
    let receipt = PublicationReceipt::new(
        PublicationChannel::Telegram,
        SnapshotId::new("snapshot-1").unwrap(),
        "2026-08-17T15:00:00Z",
        vec![TelegramMessageId::new("message-1").unwrap()],
        PublicationStatus::Published,
        1,
    )
    .unwrap();

    assert_eq!(receipt.channel(), PublicationChannel::Telegram);
    assert_eq!(receipt.snapshot_id().as_str(), "snapshot-1");
    assert_eq!(receipt.published_at(), "2026-08-17T15:00:00Z");
    assert_eq!(receipt.message_ids()[0].as_str(), "message-1");
    assert_eq!(receipt.attempt(), 1);
}
