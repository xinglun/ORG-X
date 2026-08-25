use chrono::NaiveDate;
use org_x::features::weekly_radar::runtime::evidence::{
    validate_evidence_candidate, EvidenceCandidate, EvidencePolarity, EvidenceSourceKind,
};
use org_x::features::weekly_radar::runtime::SourceTier;

#[test]
fn validated_candidate_keeps_the_evidence_identity_boundary() {
    let candidate = EvidenceCandidate::new(
        "acme",
        "Acme Corporation",
        "Acme moved its engineering workflow to production scheduling.",
        Some(NaiveDate::from_ymd_opt(2026, 8, 19).unwrap()),
        "engineering workflow",
        EvidenceSourceKind::OfficialMaterial,
        SourceTier::OfficialPrimary,
        EvidencePolarity::Supporting,
        "https://ir.example.test/organization/update",
    )
    .unwrap()
    .with_source_details(
        "Organization update",
        "Acme moved its engineering workflow to production scheduling.",
    );

    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .unwrap();

    assert!(validated
        .to_normalized_fact(1)
        .unwrap()
        .kind()
        .starts_with("evidence_"));
}
