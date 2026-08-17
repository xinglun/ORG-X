use super::{CompanyReference, Distance, EvidenceId, StageLabel, ThresholdDistance};

#[test]
fn module_local_test_preserves_supplied_distance_without_stage_comparison() {
    let threshold = ThresholdDistance::new(
        CompanyReference::new("company").expect("company should be valid"),
        StageLabel::new("REFERENCE_MODEL").expect("current stage should be valid"),
        StageLabel::new("TOOL").expect("next stage should be valid"),
        vec![EvidenceId::new("confirmed").expect("evidence should be valid")],
        vec![EvidenceId::new("missing").expect("evidence should be valid")],
        Distance::Developing,
    )
    .expect("the upstream supplied value should be retained");

    assert_eq!(threshold.distance(), Distance::Developing);
}
