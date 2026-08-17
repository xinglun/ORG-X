use org_x::features::weekly_radar::domain::system_health::{
    CompanyReference, EvidenceCoverage, Freshness, HealthStatus, SystemHealth,
};

#[test]
fn supplied_status_coverage_and_freshness_are_retained_without_inference() {
    let coverage = EvidenceCoverage::new(1, 2, 99).unwrap();
    let health = SystemHealth::new(HealthStatus::Healthy, coverage.clone(), Freshness::Stale);

    assert_eq!(health.status(), HealthStatus::Healthy);
    assert_eq!(health.evidence_coverage(), &coverage);
    assert_eq!(health.freshness(), Freshness::Stale);
    assert_eq!(
        CompanyReference::new("company-a").unwrap().as_str(),
        "company-a"
    );
}
