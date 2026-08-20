#[test]
fn follow_up_horizons_are_fixed_and_ordered() {
    assert_eq!(
        super::ValidationHorizon::FOLLOW_UPS,
        [
            super::ValidationHorizon::SixMonths,
            super::ValidationHorizon::TwelveMonths,
            super::ValidationHorizon::TwentyFourMonths,
        ]
    );
}
