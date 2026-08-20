use chrono::{NaiveDate, Utc};
use org_x::features::weekly_radar::runtime::judgment::{
    derive_judgment_snapshot, derive_judgment_snapshot_for_companies, HumanReference, MachineStage,
};
use org_x::features::weekly_radar::runtime::model::{
    Confidence, FactStatus, NormalizedFact, Provenance, RuntimeReportInput,
};
use org_x::features::weekly_radar::runtime::report::render_report;

fn fact(company_id: &str, kind: &str, value: &str, source_uri: &str) -> NormalizedFact {
    NormalizedFact::new(
        company_id,
        kind,
        value,
        FactStatus::Known,
        Confidence::High,
        Provenance::new(
            source_uri,
            "fixture field",
            Utc::now(),
            Some(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()),
        )
        .unwrap(),
    )
    .unwrap()
}

fn missing_fact(company_id: &str, stage: &str, requirement: &str) -> NormalizedFact {
    NormalizedFact::without_value(
        company_id,
        format!("judgment.missing.{stage}.{requirement}"),
        FactStatus::Unknown,
        Confidence::Unknown,
        Provenance::new(
            "fixture://missing-proof",
            "missing proof inventory",
            Utc::now(),
            None,
        )
        .unwrap(),
    )
    .unwrap()
}

fn workflow_facts(company_id: &str) -> Vec<NormalizedFact> {
    vec![
        fact(
            company_id,
            "judgment.supporting.WORKFLOW.workflow_rewrite",
            "workflow responsibility changed",
            "https://source-a.example/workflow",
        ),
        fact(
            company_id,
            "judgment.supporting.WORKFLOW.human_supervision",
            "human supervision retained",
            "https://source-b.example/operations",
        ),
        fact(
            company_id,
            "judgment.counter.WORKFLOW.counter_signal",
            "legacy workflow remains for one region",
            "https://source-c.example/risk",
        ),
        missing_fact(company_id, "WORKFLOW", "quarterly_persistence"),
    ]
}

#[test]
fn automatic_machine_reference_flows_through_evidence_stage_ranking_and_snapshot() {
    let facts = workflow_facts("acme");
    let human = HumanReference::new(
        "acme",
        "PRODUCTION_SYSTEM",
        "人的独立判断：核心生产系统仍需进一步核验。",
        "2026-08-20T10:00:00Z",
    )
    .unwrap();

    let snapshot = derive_judgment_snapshot(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        &facts,
        vec![human],
    )
    .unwrap();
    let machine = snapshot.company("acme").unwrap();

    assert_eq!(machine.machine_stage(), &MachineStage::assigned("WORKFLOW"));
    assert_eq!(machine.ranked_candidates().len(), 1);
    assert_eq!(machine.ranked_candidates()[0].stage(), "WORKFLOW");
    assert_eq!(
        snapshot.human_reference("acme").unwrap().stage(),
        "PRODUCTION_SYSTEM"
    );
    assert_ne!(
        machine.machine_stage().label(),
        snapshot.human_reference("acme").unwrap().stage()
    );
    assert!(!machine.supporting_proof().is_empty());
    assert!(!machine.counter_proof().is_empty());
    assert!(!machine.missing_proof().is_empty());
}

#[test]
fn ranking_is_isolated_to_the_machine_stage_and_does_not_merge_human_reference() {
    let mut facts = workflow_facts("acme");
    facts.extend(workflow_facts("beta").into_iter().map(|fact| {
        let kind = fact.kind().replace("WORKFLOW", "TOOL");
        match fact.value() {
            Some(value) => NormalizedFact::new(
                "beta",
                kind,
                value,
                *fact.status(),
                *fact.confidence(),
                fact.provenance().clone(),
            )
            .unwrap(),
            None => NormalizedFact::without_value(
                "beta",
                kind,
                *fact.status(),
                *fact.confidence(),
                fact.provenance().clone(),
            )
            .unwrap(),
        }
    }));

    let snapshot = derive_judgment_snapshot(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        &facts,
        Vec::new(),
    )
    .unwrap();

    let acme = snapshot.company("acme").unwrap();
    let beta = snapshot.company("beta").unwrap();
    assert_eq!(acme.machine_stage(), &MachineStage::assigned("WORKFLOW"));
    assert_eq!(beta.machine_stage(), &MachineStage::assigned("TOOL"));
    assert!(acme
        .ranked_candidates()
        .iter()
        .all(|candidate| candidate.stage() == "WORKFLOW"));
    assert!(beta
        .ranked_candidates()
        .iter()
        .all(|candidate| candidate.stage() == "TOOL"));
    assert!(acme
        .ranked_candidates()
        .iter()
        .all(|candidate| candidate.company() == "acme"));
    assert_eq!(snapshot.ranked_within_stage("WORKFLOW").len(), 1);
    assert_eq!(snapshot.ranked_within_stage("TOOL").len(), 1);
}

#[test]
fn insufficient_evidence_is_undetermined_and_has_no_machine_ranking() {
    let facts = vec![fact(
        "acme",
        "judgment.supporting.WORKFLOW.only_one_source",
        "one source is not enough",
        "https://source-a.example/workflow",
    )];

    let snapshot = derive_judgment_snapshot(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        &facts,
        Vec::new(),
    )
    .unwrap();
    let machine = snapshot.company("acme").unwrap();

    assert!(matches!(
        machine.machine_stage(),
        MachineStage::Undetermined { .. }
    ));
    assert!(machine.ranked_candidates().is_empty());
}

#[test]
fn report_serializes_validated_parallel_views_without_recomputing_them() {
    let facts = workflow_facts("acme");
    let human = HumanReference::new(
        "acme",
        "TOOL",
        "人的参考与系统不同。",
        "2026-08-20T10:00:00Z",
    )
    .unwrap();
    let judgment = derive_judgment_snapshot(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        &facts,
        vec![human],
    )
    .unwrap();
    let mut input = RuntimeReportInput::from_date(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap());
    for item in facts {
        input.add_fact(item).unwrap();
    }
    input.set_judgment(judgment).unwrap();

    let first = render_report(&input);
    let second = render_report(&input);
    assert_eq!(first.snapshot_json(), second.snapshot_json());
    assert!(first.snapshot_json().contains("machine_stage"));
    assert!(first.snapshot_json().contains("human_reference"));
    assert!(first.markdown().contains("人的参考"));
}

#[test]
fn source_observations_without_explicit_stage_signals_are_visible_as_undetermined() {
    let facts = vec![fact(
        "acme",
        "source_official_ir_001",
        "official company page",
        "https://example.test/ir",
    )];

    let snapshot = derive_judgment_snapshot_for_companies(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        ["acme"],
        &facts,
        Vec::new(),
    )
    .unwrap();

    let machine = snapshot.company("acme").unwrap();
    assert_eq!(machine.machine_stage().label(), "UNDETERMINED");
    assert!(machine.ranked_candidates().is_empty());
}

#[test]
fn companies_without_facts_are_retained_as_undetermined() {
    let snapshot = derive_judgment_snapshot_for_companies(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        ["acme", "no-facts"],
        &workflow_facts("acme"),
        Vec::new(),
    )
    .unwrap();

    let machine = snapshot.company("no-facts").unwrap();
    assert_eq!(machine.machine_stage().label(), "UNDETERMINED");
    assert!(machine.ranked_candidates().is_empty());
}

#[test]
fn future_effective_facts_cannot_satisfy_the_cutoff_gate() {
    let future = NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();
    let facts = workflow_facts("acme")
        .into_iter()
        .map(|fact| {
            let provenance = Provenance::new(
                fact.provenance().source_uri(),
                fact.provenance().source_field_or_passage(),
                *fact.provenance().retrieved_at(),
                Some(future),
            )
            .unwrap();
            match fact.value() {
                Some(value) => NormalizedFact::new(
                    fact.company_id(),
                    fact.kind(),
                    value,
                    *fact.status(),
                    *fact.confidence(),
                    provenance,
                )
                .unwrap(),
                None => NormalizedFact::without_value(
                    fact.company_id(),
                    fact.kind(),
                    *fact.status(),
                    *fact.confidence(),
                    provenance,
                )
                .unwrap(),
            }
        })
        .collect::<Vec<_>>();

    let snapshot = derive_judgment_snapshot(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        &facts,
        Vec::new(),
    )
    .unwrap();

    assert_eq!(
        snapshot.company("acme").unwrap().machine_stage().label(),
        "UNDETERMINED"
    );
}

#[test]
fn deserialization_rejects_a_judgment_that_does_not_match_input_as_of() {
    let facts = workflow_facts("acme");
    let judgment = derive_judgment_snapshot(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        &facts,
        Vec::new(),
    )
    .unwrap();
    let mut input = RuntimeReportInput::from_date(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap());
    for item in facts {
        input.add_fact(item).unwrap();
    }
    input.set_judgment(judgment).unwrap();

    let mut wire = serde_json::to_value(&input).unwrap();
    wire["judgment"]["evidence_cutoff"] = serde_json::json!("2026-08-21");

    assert!(serde_json::from_value::<RuntimeReportInput>(wire).is_err());
}
