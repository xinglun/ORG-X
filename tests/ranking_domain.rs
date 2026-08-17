use org_x::features::ranking::domain::{
    CompanyReference, CounterEvidenceRisk, EvidenceConfidence, EvidenceFreshness, RankingCandidate,
    RankingCandidateId, RankingReadModel, Stage, TransformationScore,
};

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
        RankingCandidateId::new(identity).expect("candidate identity should be valid"),
        CompanyReference::new(company_name).expect("company reference should be valid"),
        stage,
        EvidenceConfidence::new(confidence).expect("confidence should be valid"),
        TransformationScore::new(score).expect("score should be valid"),
        CounterEvidenceRisk::new(risk).expect("risk should be valid"),
        EvidenceFreshness::new(freshness).expect("freshness should be valid"),
    )
    .expect("candidate should be valid")
}

#[test]
fn public_ranking_api_orders_only_the_requested_stage() {
    let mut model = RankingReadModel::new();
    model
        .add(candidate(
            "workflow-1",
            "alpha",
            Stage::Workflow,
            80,
            60,
            10,
            90,
        ))
        .expect("candidate is accepted");
    model
        .add(candidate(
            "workflow-2",
            "beta",
            Stage::Workflow,
            80,
            60,
            20,
            90,
        ))
        .expect("candidate is accepted");
    model
        .add(candidate(
            "reference-1",
            "reference-co",
            Stage::ReferenceModel,
            100,
            100,
            0,
            100,
        ))
        .expect("candidate is accepted");

    let ranked = model.ranked_within_stage(Stage::Workflow);

    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].id().as_str(), "workflow-1");
    assert_eq!(ranked[1].id().as_str(), "workflow-2");
    assert_eq!(model.ranked_within_stage(Stage::ReferenceModel).len(), 1);
}
