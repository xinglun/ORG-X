use super::*;

#[test]
fn observation_contract_preserves_provenance_and_payload() {
    let observation = Observation::new(
        ObservationId::new("observation-1").unwrap(),
        SourceUri::new("https://example.test/filing").unwrap(),
        SourceTitle::new("Annual filing").unwrap(),
        ObservationTime::new("2026-08-17T03:00:00Z").unwrap(),
        Some(EffectiveDate::new("2026-06-30").unwrap()),
        ContentHash::new("sha256:abc").unwrap(),
        SourceTier::A,
        ObservationKind::Filing,
        vec![1, 2, 3],
    )
    .unwrap();

    assert_eq!(observation.id().as_str(), "observation-1");
    assert_eq!(
        observation.source_uri().as_str(),
        "https://example.test/filing"
    );
    assert_eq!(observation.source_title().as_str(), "Annual filing");
    assert_eq!(observation.observed_at().as_str(), "2026-08-17T03:00:00Z");
    assert_eq!(observation.effective_date().unwrap().as_str(), "2026-06-30");
    assert_eq!(observation.content_hash().as_str(), "sha256:abc");
    assert_eq!(observation.source_tier(), &SourceTier::A);
    assert_eq!(observation.kind(), &ObservationKind::Filing);
    assert_eq!(observation.payload(), &[1, 2, 3]);
}

#[test]
fn receipt_rejects_duplicate_observation_identity() {
    let mut receipt = IngestionReceipt::new(
        IngestionReceiptId::new("receipt-1").unwrap(),
        ObservationTime::new("2026-08-17T03:01:00Z").unwrap(),
    );
    let observation = Observation::new(
        ObservationId::new("observation-1").unwrap(),
        SourceUri::new("https://example.test/filing").unwrap(),
        SourceTitle::new("Annual filing").unwrap(),
        ObservationTime::new("2026-08-17T03:00:00Z").unwrap(),
        None,
        ContentHash::new("sha256:abc").unwrap(),
        SourceTier::A,
        ObservationKind::Filing,
        vec![],
    )
    .unwrap();

    receipt.accept(observation.clone()).unwrap();
    assert!(matches!(
        receipt.accept(observation),
        Err(IngestionDomainError::DuplicateObservationId { .. })
    ));
}
