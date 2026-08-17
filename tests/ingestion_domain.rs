use org_x::features::ingestion::application::{
    IngestionRequest, ObservationSource, SourceCollectionError,
};
use org_x::features::ingestion::domain::{
    ContentHash, IngestionDomainError, IngestionReceipt, IngestionReceiptId, Observation,
    ObservationId, ObservationKind, ObservationTime, SourceTier, SourceTitle, SourceUri,
};

fn observation(id: &str, payload: &[u8]) -> Observation {
    Observation::new(
        ObservationId::new(id).unwrap(),
        SourceUri::new(format!("https://example.test/{id}")).unwrap(),
        SourceTitle::new(format!("Source {id}")).unwrap(),
        ObservationTime::new("2026-08-17T03:00:00Z").unwrap(),
        None,
        ContentHash::new(format!("sha256:{id}")).unwrap(),
        SourceTier::B,
        ObservationKind::EngineeringMaterial,
        payload.to_vec(),
    )
    .unwrap()
}

#[test]
fn required_metadata_rejects_blank_values() {
    assert!(matches!(
        ObservationId::new("  "),
        Err(IngestionDomainError::EmptyValue {
            field: "observation id"
        })
    ));
    assert!(matches!(
        SourceUri::new(""),
        Err(IngestionDomainError::EmptyValue {
            field: "source uri"
        })
    ));
    assert!(matches!(
        SourceTitle::new("\n"),
        Err(IngestionDomainError::EmptyValue {
            field: "source title"
        })
    ));
    assert!(matches!(
        ObservationTime::new(""),
        Err(IngestionDomainError::EmptyValue {
            field: "observation time"
        })
    ));
    assert!(matches!(
        ContentHash::new(""),
        Err(IngestionDomainError::EmptyValue {
            field: "content hash"
        })
    ));
}

#[test]
fn receipt_preserves_insertion_order_and_identity_view() {
    let mut receipt = IngestionReceipt::new(
        IngestionReceiptId::new("receipt-1").unwrap(),
        ObservationTime::new("2026-08-17T03:01:00Z").unwrap(),
    );
    receipt.accept(observation("first", b"one")).unwrap();
    receipt.accept(observation("second", b"two")).unwrap();

    let ids: Vec<_> = receipt
        .observation_ids()
        .iter()
        .map(|id| id.as_str())
        .collect();
    assert_eq!(ids, vec!["first", "second"]);
    assert_eq!(receipt.observations()[0].payload(), b"one");
    assert_eq!(receipt.observations()[1].payload(), b"two");
}

#[test]
fn source_collection_port_returns_opaque_observations() {
    struct FixtureSource;

    impl ObservationSource for FixtureSource {
        fn collect(
            &self,
            _request: &IngestionRequest,
        ) -> Result<Vec<Observation>, SourceCollectionError> {
            Ok(vec![observation("fixture", b"uninterpreted")])
        }
    }

    let observations = FixtureSource
        .collect(&IngestionRequest::new("fixture"))
        .unwrap();
    assert_eq!(observations[0].payload(), b"uninterpreted");
}
