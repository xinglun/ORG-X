use org_x::features::weekly_radar::domain::system_health::{
    EvidenceCoverage, Freshness, HealthStatus, SystemHealth,
};

#[test]
fn weekly_radar_domain_module_exports_explicit_health_facts() {
    let health = SystemHealth::new(
        HealthStatus::Unknown,
        EvidenceCoverage::new(0, 1, 0).unwrap(),
        Freshness::Unknown,
    );

    assert_eq!(health.status(), HealthStatus::Unknown);
    assert_eq!(health.freshness(), Freshness::Unknown);
}
