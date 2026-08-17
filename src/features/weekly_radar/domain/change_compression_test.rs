use super::*;

fn period(value: &str) -> PeriodId {
    PeriodId::new(value).expect("period should be valid")
}

fn event(value: &str) -> EventId {
    EventId::new(value).expect("event should be valid")
}

fn company(value: &str) -> CompanyReference {
    CompanyReference::new(value).expect("company should be valid")
}

fn fact(value: &str) -> FactValue {
    FactValue::new(value).expect("fact should be valid")
}

#[test]
fn module_local_test_preserves_explicit_values_and_stable_empty_output() {
    let input = WeeklyChangeInput::new(
        period("2026-W33"),
        vec![
            ImportantStructuralChange::new(
                event("one"),
                period("2026-W33"),
                company("Acme"),
                fact("first supplied fact"),
            )
            .expect("event should be valid"),
            ImportantStructuralChange::new(
                event("two"),
                period("2026-W33"),
                company("Beta"),
                fact("second supplied fact"),
            )
            .expect("event should be valid"),
        ],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("input should be valid");

    let compression = WeeklyChangeCompression::from_input(input).expect("compression should work");
    assert_eq!(
        compression.important_structural()[0].event_id().as_str(),
        "one"
    );
    assert_eq!(
        compression.important_structural()[1].fact().as_str(),
        "second supplied fact"
    );
    assert!(compression.no_change().is_none());

    let empty = WeeklyChangeCompression::from_input(
        WeeklyChangeInput::new(
            period("2026-W34"),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("empty input should be valid"),
    )
    .expect("empty compression should work");
    assert_eq!(
        empty.no_change().expect("stable output").label(),
        "NO_CHANGE"
    );
    assert_eq!(empty.sections().len(), 6);
}

#[test]
fn module_local_test_rejects_duplicate_identity_and_period_mismatch() {
    let duplicate = WeeklyChangeInput::new(
        period("2026-W33"),
        vec![ImportantStructuralChange::new(
            event("same"),
            period("2026-W33"),
            company("Acme"),
            fact("first"),
        )
        .expect("event should be valid")],
        vec![Top5Change::new(
            event("same"),
            period("2026-W33"),
            company("Beta"),
            fact("second"),
        )
        .expect("event should be valid")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect_err("duplicate identity should fail");
    assert_eq!(
        duplicate,
        ChangeCompressionError::DuplicateIdentity {
            id: "same".to_owned(),
        }
    );

    let mismatch = WeeklyChangeInput::new(
        period("2026-W33"),
        vec![ImportantStructuralChange::new(
            event("mismatch"),
            period("2026-W34"),
            company("Acme"),
            fact("fact"),
        )
        .expect("event should be valid")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect_err("period mismatch should fail");
    assert_eq!(
        mismatch,
        ChangeCompressionError::PeriodMismatch {
            expected: "2026-W33".to_owned(),
            actual: "2026-W34".to_owned(),
        }
    );
}
