use super::*;

fn id(value: &str) -> RankingCandidateId {
    RankingCandidateId::new(value).expect("candidate identity should be valid")
}

fn company(value: &str) -> CompanyReference {
    CompanyReference::new(value).expect("company reference should be valid")
}

fn candidate(
    identity: &str,
    company_name: &str,
    stage: Stage,
    confidence: u8,
    score: u8,
    risk: u8,
    freshness: u8,
) -> RankingCandidate {
    RankingCandidate::new(
        id(identity),
        company(company_name),
        stage,
        EvidenceConfidence::new(confidence).expect("confidence should be valid"),
        TransformationScore::new(score).expect("score should be valid"),
        CounterEvidenceRisk::new(risk).expect("risk should be valid"),
        EvidenceFreshness::new(freshness).expect("freshness should be valid"),
    )
    .expect("candidate should be valid")
}

#[test]
fn ranking_uses_fixed_key_order_with_deterministic_ties() {
    let mut model = RankingReadModel::new();
    model
        .add(candidate("b", "beta", Stage::Workflow, 90, 80, 20, 70))
        .expect("candidate is accepted");
    model
        .add(candidate("a", "alpha", Stage::Workflow, 90, 80, 20, 70))
        .expect("candidate is accepted");
    model
        .add(candidate("c", "gamma", Stage::Workflow, 95, 1, 99, 1))
        .expect("candidate is accepted");

    let ranked = model.ranked_within_stage(Stage::Workflow);
    let identities: Vec<_> = ranked.iter().map(|item| item.id().as_str()).collect();

    assert_eq!(identities, vec!["c", "a", "b"]);
}

#[test]
fn ranking_keeps_stages_isolated() {
    let mut model = RankingReadModel::new();
    model
        .add(candidate("tool", "tool-co", Stage::Tool, 1, 1, 1, 1))
        .expect("candidate is accepted");
    model
        .add(candidate(
            "production",
            "production-co",
            Stage::ProductionSystem,
            100,
            100,
            0,
            100,
        ))
        .expect("candidate is accepted");

    assert_eq!(model.ranked_within_stage(Stage::Tool).len(), 1);
    assert_eq!(
        model.ranked_within_stage(Stage::Tool)[0].id().as_str(),
        "tool"
    );
    assert_eq!(model.ranked_within_stage(Stage::ProductionSystem).len(), 1);
    assert!(model.ranked_within_stage(Stage::Substitution).is_empty());
}

#[test]
fn ranking_rejects_invalid_values_and_duplicate_candidates() {
    assert!(matches!(
        CompanyReference::new("  "),
        Err(RankingDomainError::EmptyValue { .. })
    ));
    assert!(matches!(
        EvidenceConfidence::new(101),
        Err(RankingDomainError::OutOfRange { .. })
    ));

    let mut model = RankingReadModel::new();
    let first = candidate("same", "alpha", Stage::Workflow, 50, 50, 50, 50);
    model
        .add(first.clone())
        .expect("first candidate is accepted");
    assert_eq!(
        model.add(first),
        Err(RankingDomainError::DuplicateIdentity {
            entity: "ranking candidate",
            id: "same".to_owned(),
        })
    );
}
