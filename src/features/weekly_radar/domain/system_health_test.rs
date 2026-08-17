use super::*;

fn health() -> SystemHealth {
    SystemHealth::new(
        HealthStatus::Degraded,
        EvidenceCoverage::new(3, 3, 100).unwrap(),
        Freshness::Current,
    )
}

fn source(value: &str) -> SourceReference {
    SourceReference::new(value).unwrap()
}

fn reason(value: &str) -> Reason {
    Reason::new(value).unwrap()
}

#[test]
fn blank_text_and_out_of_range_percentage_are_rejected() {
    assert_eq!(
        CompanyReference::new("  "),
        Err(SystemHealthDomainError::EmptyValue {
            field: "company reference"
        })
    );
    assert_eq!(
        EvidenceCoverage::new(1, 2, 101),
        Err(SystemHealthDomainError::InvalidPercentage { value: 101 })
    );
}

#[test]
fn source_coverage_preserves_order_and_rejects_duplicate_identity() {
    let mut health = health();
    health
        .add_source_coverage(SourceCoverage::new(source("filing"), 1, 2, 50).unwrap())
        .unwrap();
    health
        .add_source_coverage(SourceCoverage::new(source("official"), 2, 2, 80).unwrap())
        .unwrap();

    assert_eq!(
        health
            .source_coverage()
            .iter()
            .map(|item| item.source().as_str())
            .collect::<Vec<_>>(),
        ["filing", "official"]
    );
    assert_eq!(
        health.add_source_coverage(SourceCoverage::new(source("filing"), 2, 2, 100).unwrap()),
        Err(SystemHealthDomainError::DuplicateIdentity {
            entity: "source coverage",
            id: "filing".to_owned(),
        })
    );
    assert_eq!(health.source_coverage().len(), 2);
}

#[test]
fn degraded_companies_and_extraction_failures_retain_order_and_reject_duplicates() {
    let mut health = health();
    health
        .add_degraded_company(DegradedCompany::new(
            CompanyReference::new("company-a").unwrap(),
            reason("stale source"),
        ))
        .unwrap();
    health
        .add_degraded_company(DegradedCompany::new(
            CompanyReference::new("company-b").unwrap(),
            reason("failed extraction"),
        ))
        .unwrap();
    health
        .add_extraction_failure(ExtractionFailure::new(
            FailureId::new("failure-a").unwrap(),
            source("filing"),
            reason("parse error"),
        ))
        .unwrap();

    assert_eq!(health.degraded_companies().len(), 2);
    assert_eq!(
        health.degraded_companies()[0].company().as_str(),
        "company-a"
    );
    assert_eq!(health.extraction_failures()[0].id().as_str(), "failure-a");
    assert_eq!(
        health.add_degraded_company(DegradedCompany::new(
            CompanyReference::new("company-a").unwrap(),
            reason("replacement"),
        )),
        Err(SystemHealthDomainError::DuplicateIdentity {
            entity: "degraded company",
            id: "company-a".to_owned(),
        })
    );
    assert_eq!(
        health.add_extraction_failure(ExtractionFailure::new(
            FailureId::new("failure-a").unwrap(),
            source("official"),
            reason("replacement"),
        )),
        Err(SystemHealthDomainError::DuplicateIdentity {
            entity: "extraction failure",
            id: "failure-a".to_owned(),
        })
    );
}

#[test]
fn explicit_status_is_not_recomputed_from_full_coverage() {
    let health = health();

    assert_eq!(health.status(), HealthStatus::Degraded);
    assert_eq!(health.evidence_coverage().percentage().value(), 100);
    assert_eq!(health.freshness(), Freshness::Current);
}
