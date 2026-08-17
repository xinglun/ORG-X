use super::*;

fn record(id: &str, company: &str, polarity: EvidencePolarity) -> EvidenceRecord {
    EvidenceRecord::new(
        EvidenceId::new(id).unwrap(),
        CompanyReference::new(company).unwrap(),
        ObservationTime::new("2026-08-17T04:00:00Z").unwrap(),
        Some(EffectiveDate::new("2026-06-30").unwrap()),
        EvidenceType::Operational,
        SourceType::OfficialMaterial,
        SourceUri::new("https://example.test/evidence").unwrap(),
        SourceTitle::new("Operating update").unwrap(),
        Claim::new("The operating workflow changed").unwrap(),
        Some(NormalizedValue::new("changed").unwrap()),
        polarity,
        Confidence::High,
        Freshness::Current,
        ExtractorVersion::new("extractor-1").unwrap(),
        ContentHash::new("sha256:record").unwrap(),
    )
    .unwrap()
}

#[test]
fn evidence_record_preserves_provenance_and_quality() {
    let evidence = record("evidence-1", "company-1", EvidencePolarity::Supporting);

    assert_eq!(evidence.id().as_str(), "evidence-1");
    assert_eq!(evidence.company_id().as_str(), "company-1");
    assert_eq!(evidence.observed_at().as_str(), "2026-08-17T04:00:00Z");
    assert_eq!(evidence.effective_date().unwrap().as_str(), "2026-06-30");
    assert_eq!(
        evidence.source_uri().as_str(),
        "https://example.test/evidence"
    );
    assert_eq!(evidence.source_title().as_str(), "Operating update");
    assert_eq!(evidence.claim().as_str(), "The operating workflow changed");
    assert_eq!(evidence.normalized_value().unwrap().as_str(), "changed");
    assert_eq!(evidence.polarity(), &EvidencePolarity::Supporting);
    assert_eq!(evidence.confidence(), &Confidence::High);
    assert_eq!(evidence.freshness(), &Freshness::Current);
    assert_eq!(evidence.extractor_version().as_str(), "extractor-1");
    assert_eq!(evidence.content_hash().as_str(), "sha256:record");
}

#[test]
fn evidence_set_routes_and_rejects_invalid_members() {
    let company = CompanyReference::new("company-1").unwrap();
    let mut set = EvidenceSet::new(company);
    let supporting = record("supporting", "company-1", EvidencePolarity::Supporting);
    let counter = record("counter", "company-1", EvidencePolarity::Counter);

    set.add(supporting.clone()).unwrap();
    set.add(counter).unwrap();
    set.add_missing(
        MissingEvidence::new(
            MissingEvidenceId::new("missing-1").unwrap(),
            Claim::new("Current headcount disclosure").unwrap(),
            MissingReason::Unavailable,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(set.supporting(), std::slice::from_ref(&supporting));
    assert_eq!(set.counter().len(), 1);
    assert_eq!(set.missing().len(), 1);
    assert!(matches!(
        set.add(supporting),
        Err(EvidenceDomainError::DuplicateEvidenceId { .. })
    ));
    assert!(matches!(
        set.add(record(
            "wrong-company",
            "company-2",
            EvidencePolarity::Supporting
        )),
        Err(EvidenceDomainError::CompanyMismatch { .. })
    ));
}
