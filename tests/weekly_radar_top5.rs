#[path = "../src/features/weekly_radar/domain/top5_weekly_read_model.rs"]
mod top5_weekly_read_model;

use top5_weekly_read_model::{
    CandidateId, Company, Confidence, Direction, KeyChange, NextStep, Stage, Top5DomainError,
    Top5Entry, Top5WeeklyReadModel,
};

fn entry(id: &str) -> Top5Entry {
    Top5Entry::new(
        CandidateId::new(id).unwrap(),
        Company::new(format!("company-{id}")).unwrap(),
        Stage::new(format!("stage-{id}")).unwrap(),
        Direction::new(format!("direction-{id}")).unwrap(),
        Confidence::new(format!("confidence-{id}")).unwrap(),
        KeyChange::new(format!("change-{id}")).unwrap(),
        NextStep::new(format!("next-{id}")).unwrap(),
    )
    .unwrap()
}

#[test]
fn entry_retains_all_supplied_facts_without_normalizing_them() {
    let entry = Top5Entry::new(
        CandidateId::new("candidate-1").unwrap(),
        Company::new("  Acme Labs  ").unwrap(),
        Stage::new("PRODUCTIVITY_BREAKOUT").unwrap(),
        Direction::new("UP").unwrap(),
        Confidence::new("HIGH").unwrap(),
        KeyChange::new("  workflow transfer strengthened  ").unwrap(),
        NextStep::new("validate persistence").unwrap(),
    )
    .unwrap();

    assert_eq!(entry.candidate().as_str(), "candidate-1");
    assert_eq!(entry.company().as_str(), "  Acme Labs  ");
    assert_eq!(entry.stage().as_str(), "PRODUCTIVITY_BREAKOUT");
    assert_eq!(entry.direction().as_str(), "UP");
    assert_eq!(entry.confidence().as_str(), "HIGH");
    assert_eq!(
        entry.key_change().as_str(),
        "  workflow transfer strengthened  "
    );
    assert_eq!(entry.next().as_str(), "validate persistence");
}

#[test]
fn blank_values_are_rejected_before_an_entry_is_created() {
    assert_eq!(
        CandidateId::new(" \t "),
        Err(Top5DomainError::EmptyValue { field: "candidate" })
    );
}

#[test]
fn read_model_accepts_empty_input_and_preserves_supplied_order() {
    let empty = Top5WeeklyReadModel::new();
    assert!(empty.is_empty());

    let model = Top5WeeklyReadModel::from_entries(vec![
        entry("candidate-3"),
        entry("candidate-1"),
        entry("candidate-2"),
    ])
    .unwrap();

    assert_eq!(model.len(), 3);
    assert_eq!(
        model
            .entries()
            .iter()
            .map(|item| item.candidate().as_str())
            .collect::<Vec<_>>(),
        ["candidate-3", "candidate-1", "candidate-2"]
    );
}

#[test]
fn read_model_accepts_five_entries_and_rejects_a_sixth_without_mutation() {
    let mut model = Top5WeeklyReadModel::new();
    for index in 1..=5 {
        model.add(entry(&format!("candidate-{index}"))).unwrap();
    }

    assert_eq!(
        model.add(entry("candidate-6")),
        Err(Top5DomainError::Top5LimitExceeded { limit: 5 })
    );
    assert_eq!(model.len(), 5);
    assert_eq!(model.entries()[4].candidate().as_str(), "candidate-5");
}

#[test]
fn duplicate_candidate_is_rejected_before_capacity_and_does_not_mutate_model() {
    let mut model = Top5WeeklyReadModel::new();
    for index in 1..=5 {
        model.add(entry(&format!("candidate-{index}"))).unwrap();
    }

    let duplicate = Top5Entry::new(
        CandidateId::new("candidate-1").unwrap(),
        Company::new("new-company").unwrap(),
        Stage::new("new-stage").unwrap(),
        Direction::new("new-direction").unwrap(),
        Confidence::new("new-confidence").unwrap(),
        KeyChange::new("new-change").unwrap(),
        NextStep::new("new-next").unwrap(),
    )
    .unwrap();

    assert_eq!(
        model.add(duplicate),
        Err(Top5DomainError::DuplicateIdentity {
            entity: "top5 candidate",
            id: "candidate-1".to_owned(),
        })
    );
    assert_eq!(model.len(), 5);
    assert_eq!(model.entries()[0].company().as_str(), "company-candidate-1");
}
