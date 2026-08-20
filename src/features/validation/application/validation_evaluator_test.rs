#[test]
fn readiness_has_only_completeness_states() {
    assert_ne!(
        super::ValidationReadiness::Complete,
        super::ValidationReadiness::Incomplete
    );
}
