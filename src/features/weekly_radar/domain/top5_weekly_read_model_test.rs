use super::*;

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
fn blank_candidate_is_rejected() {
    assert_eq!(
        CandidateId::new("   "),
        Err(Top5DomainError::EmptyValue { field: "candidate" })
    );
}

#[test]
fn from_entries_keeps_input_order() {
    let model = Top5WeeklyReadModel::from_entries([entry("third"), entry("first")]).unwrap();

    assert_eq!(
        model
            .entries()
            .iter()
            .map(|item| item.candidate().as_str())
            .collect::<Vec<_>>(),
        ["third", "first"]
    );
}
