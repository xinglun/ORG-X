use org_x::features::evidence::domain::{
    Claim, CompanyReference, ContentHash, EvidenceDomainError, EvidenceId, ExtractorVersion,
    ObservationTime, SourceTitle, SourceUri,
};

#[test]
fn required_evidence_fields_reject_blank_values() {
    assert!(matches!(
        EvidenceId::new(" "),
        Err(EvidenceDomainError::EmptyValue {
            field: "evidence id"
        })
    ));
    assert!(matches!(
        CompanyReference::new(""),
        Err(EvidenceDomainError::EmptyValue {
            field: "company reference"
        })
    ));
    assert!(matches!(
        ObservationTime::new("\n"),
        Err(EvidenceDomainError::EmptyValue {
            field: "observation time"
        })
    ));
    assert!(matches!(
        SourceUri::new(""),
        Err(EvidenceDomainError::EmptyValue {
            field: "source uri"
        })
    ));
    assert!(matches!(
        SourceTitle::new("  "),
        Err(EvidenceDomainError::EmptyValue {
            field: "source title"
        })
    ));
    assert!(matches!(
        Claim::new(""),
        Err(EvidenceDomainError::EmptyValue { field: "claim" })
    ));
    assert!(matches!(
        ExtractorVersion::new(""),
        Err(EvidenceDomainError::EmptyValue {
            field: "extractor version"
        })
    ));
    assert!(matches!(
        ContentHash::new(""),
        Err(EvidenceDomainError::EmptyValue {
            field: "content hash"
        })
    ));
}
