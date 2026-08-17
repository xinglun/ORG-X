use super::*;
use crate::features::weekly_radar::domain::SnapshotId;
use crate::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, ModelVersion, ScoringVersion, UniverseSnapshotId, WeeklyRadarSnapshot,
};
use crate::features::weekly_radar::infrastructure::publication_receipt::PublicationChannel;
use crate::features::weekly_radar::infrastructure::telegram_publisher::TelegramMessageId;

fn snapshot(id: &str) -> WeeklyRadarSnapshot {
    WeeklyRadarSnapshot::new(
        SnapshotId::new(id).expect("snapshot ID should be valid"),
        AsOf::new("2026-08-16").expect("as-of should be valid"),
        UniverseSnapshotId::new("universe-archive-test").expect("universe should be valid"),
        EvidenceCutoff::new("2026-08-15T23:59:59Z").expect("cutoff should be valid"),
        ModelVersion::new("model-archive-test").expect("model should be valid"),
        ScoringVersion::new("scoring-archive-test").expect("scoring should be valid"),
    )
    .expect("snapshot should be valid")
}

fn receipt(id: &str, status: PublicationStatus) -> PublicationReceipt {
    PublicationReceipt::new(
        PublicationChannel::Telegram,
        SnapshotId::new(id).expect("receipt snapshot ID should be valid"),
        "2026-08-17T16:00:00Z",
        vec![TelegramMessageId::new("archive-message-1").expect("message ID should be valid")],
        status,
        1,
    )
    .expect("receipt should be valid")
}

#[test]
fn published_receipt_is_appended_and_read_by_snapshot_identity() {
    let snapshot = snapshot("archive-test-1");
    let id = snapshot.id().clone();
    let published = receipt(id.as_str(), PublicationStatus::Published);
    let mut archive = InMemoryWeeklyRadarArchive::new();

    archive
        .archive(snapshot.clone(), published.clone())
        .expect("published receipt should archive");

    assert_eq!(archive.entries().len(), 1);
    assert_eq!(
        archive.get(&id).map(|entry| entry.snapshot()),
        Some(&snapshot)
    );
    assert_eq!(
        archive.get(&id).map(|entry| entry.receipt()),
        Some(&published)
    );
}

#[test]
fn archive_rejects_unpublished_mismatch_and_duplicate_without_mutation() {
    let snapshot = snapshot("archive-test-2");
    let id = snapshot.id().clone();
    let mut archive = InMemoryWeeklyRadarArchive::new();

    let partial = receipt(
        id.as_str(),
        PublicationStatus::Partial {
            failed_message_index: 1,
        },
    );
    assert_eq!(
        archive.archive(snapshot.clone(), partial),
        Err(WeeklyRadarArchiveError::ReceiptNotPublished {
            snapshot_id: id.as_str().to_owned(),
        })
    );

    let mismatch = receipt("archive-foreign", PublicationStatus::Published);
    assert!(matches!(
        archive.archive(snapshot.clone(), mismatch),
        Err(WeeklyRadarArchiveError::SnapshotMismatch { .. })
    ));
    assert!(archive.entries().is_empty());

    archive
        .archive(
            snapshot.clone(),
            receipt(id.as_str(), PublicationStatus::Published),
        )
        .expect("first published receipt should archive");
    assert!(matches!(
        archive.archive(snapshot, receipt(id.as_str(), PublicationStatus::Published)),
        Err(WeeklyRadarArchiveError::DuplicateSnapshot { .. })
    ));
    assert_eq!(archive.entries().len(), 1);
}
