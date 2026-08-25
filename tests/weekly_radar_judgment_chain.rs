use chrono::{NaiveDate, Utc};
use org_x::features::transformation::domain::{
    ReferenceModelEligibility, ReferenceModelEvidenceFamily,
};
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

fn reference_fact(
    company_id: &str,
    kind: &str,
    value: &str,
    source_uri: &str,
    effective_date: &str,
    family: ReferenceModelEvidenceFamily,
    named_peer: Option<&str>,
) -> NormalizedFact {
    NormalizedFact::new_with_structural_dimension_and_reference_model_metadata(
        company_id,
        kind,
        value,
        None,
        Some(family),
        named_peer.map(str::to_owned),
        FactStatus::Known,
        Confidence::High,
        Provenance::new(
            source_uri,
            "reference-model claim",
            Utc::now(),
            Some(NaiveDate::parse_from_str(effective_date, "%Y-%m-%d").unwrap()),
        )
        .unwrap(),
    )
    .unwrap()
}

fn confirmed_reference_model_facts(company_id: &str) -> Vec<NormalizedFact> {
    vec![
        reference_fact(
            company_id,
            "judgment.supporting.REFERENCE_MODEL.organization_rewrite",
            "The company moved decision rights into an AI operating organization.",
            "https://ir.example.test/organization",
            "2026-08-10",
            ReferenceModelEvidenceFamily::OrganizationRewrite,
            None,
        ),
        reference_fact(
            company_id,
            "judgment.supporting.REFERENCE_MODEL.production_system_rewrite",
            "The company rebuilt its engineering production system around agents.",
            "https://engineering.example.test/agents",
            "2026-08-11",
            ReferenceModelEvidenceFamily::ProductionSystemRewrite,
            None,
        ),
        reference_fact(
            company_id,
            "judgment.supporting.REFERENCE_MODEL.outcome_2025",
            "Operating margin improved during the 2025 reporting period.",
            "https://www.sec.gov/Archives/2025/acme-10k",
            "2025-12-31",
            ReferenceModelEvidenceFamily::SustainedOutcome,
            None,
        ),
        reference_fact(
            company_id,
            "judgment.supporting.REFERENCE_MODEL.outcome_2026",
            "Operating margin remained improved during the 2026 reporting period.",
            "https://www.sec.gov/Archives/2026/acme-10k",
            "2026-06-30",
            ReferenceModelEvidenceFamily::SustainedOutcome,
            None,
        ),
        reference_fact(
            company_id,
            "judgment.supporting.REFERENCE_MODEL.diffusion_peer_a",
            "Peer Alpha adopted the operating model.",
            "https://peer-alpha.example/adoption",
            "2026-08-12",
            ReferenceModelEvidenceFamily::IndustryDiffusion,
            Some("Peer Alpha"),
        ),
        reference_fact(
            company_id,
            "judgment.supporting.REFERENCE_MODEL.diffusion_peer_b",
            "Peer Beta implemented the production system.",
            "https://peer-beta.example/adoption",
            "2026-08-13",
            ReferenceModelEvidenceFamily::IndustryDiffusion,
            Some("Peer Beta"),
        ),
        fact(
            company_id,
            "judgment.counter.REFERENCE_MODEL.counter_signal",
            "Legacy production remains in one region.",
            "https://risk.example.test/legacy",
        ),
        missing_fact(company_id, "REFERENCE_MODEL", "counter_review_inventory"),
    ]
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
fn reference_model_core_rewrite_is_candidate_but_cannot_enter_reference_stage() {
    let facts = vec![
        reference_fact(
            "candidate",
            "judgment.supporting.REFERENCE_MODEL.organization_rewrite",
            "The company changed reporting lines around AI.",
            "https://ir.example.test/organization",
            "2026-08-10",
            ReferenceModelEvidenceFamily::OrganizationRewrite,
            None,
        ),
        reference_fact(
            "candidate",
            "judgment.supporting.REFERENCE_MODEL.production_system_rewrite",
            "The company changed its engineering production system.",
            "https://engineering.example.test/system",
            "2026-08-11",
            ReferenceModelEvidenceFamily::ProductionSystemRewrite,
            None,
        ),
        fact(
            "candidate",
            "judgment.counter.REFERENCE_MODEL.counter_signal",
            "Legacy production remains in one region.",
            "https://risk.example.test/legacy",
        ),
        missing_fact("candidate", "REFERENCE_MODEL", "outcome_and_diffusion"),
    ];
    let snapshot = derive_judgment_snapshot_for_companies(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        ["candidate"],
        &facts,
        Vec::new(),
    )
    .unwrap();
    let machine = snapshot.company("candidate").unwrap();

    assert_eq!(
        machine.reference_model_assessment().eligibility(),
        ReferenceModelEligibility::Candidate
    );
    assert!(machine
        .reference_model_assessment()
        .missing()
        .iter()
        .any(|missing| missing == "sustained_outcome"));
    assert_eq!(machine.machine_stage().label(), "UNDETERMINED");
    assert!(machine.ranked_candidates().is_empty());
}

#[test]
fn confirmed_reference_model_assessment_is_the_only_reference_stage_gate() {
    let facts = confirmed_reference_model_facts("reference");
    let snapshot = derive_judgment_snapshot_for_companies(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        ["reference"],
        &facts,
        Vec::new(),
    )
    .unwrap();
    let machine = snapshot.company("reference").unwrap();

    assert_eq!(
        machine.reference_model_assessment().eligibility(),
        ReferenceModelEligibility::Confirmed
    );
    assert_eq!(machine.machine_stage().label(), "REFERENCE_MODEL");
    assert_eq!(machine.ranked_candidates().len(), 1);
    assert_eq!(machine.ranked_candidates()[0].stage(), "REFERENCE_MODEL");
}

#[test]
fn explicit_counter_review_marker_confirms_without_fabricating_counter_claim() {
    let mut facts = confirmed_reference_model_facts("reviewed");
    facts.retain(|fact| !fact.kind().starts_with("judgment.counter."));
    facts.push(fact(
        "reviewed",
        "judgment.review.REFERENCE_MODEL.counter_evidence_review",
        "Counter-evidence review completed across the bounded authoritative corpus; no disconfirming reference-model claim was identified.",
        "https://www.microsoft.com/en-us/ai/frontier-transformation",
    ));

    let snapshot = derive_judgment_snapshot_for_companies(
        NaiveDate::from_ymd_opt(2026, 8, 25).unwrap(),
        ["reviewed"],
        &facts,
        Vec::new(),
    )
    .unwrap();
    let assessment = snapshot
        .company("reviewed")
        .unwrap()
        .reference_model_assessment();

    assert_eq!(
        assessment.eligibility(),
        ReferenceModelEligibility::Confirmed
    );
    assert_eq!(assessment.counter_evidence_count(), 0);
    assert!(assessment.counter_reviewed());
    assert!(!assessment
        .missing()
        .iter()
        .any(|item| item == "counter_evidence_review"));
}

#[test]
fn confirmed_reference_packet_assigns_stage_without_generic_reference_signal() {
    let mut facts = confirmed_reference_model_facts("metadata-only");
    for fact in &mut facts {
        if fact.kind().starts_with("judgment.supporting.") {
            let replacement = fact.kind().replace(
                "judgment.supporting.REFERENCE_MODEL",
                "evidence_structural_change",
            );
            *fact = NormalizedFact::new_with_structural_dimension_and_reference_model_metadata(
                fact.company_id(),
                replacement,
                fact.value().unwrap_or("reference packet claim"),
                None,
                fact.reference_model_family(),
                fact.reference_model_named_peer().map(str::to_owned),
                FactStatus::Known,
                Confidence::High,
                fact.provenance().clone(),
            )
            .unwrap();
        }
    }
    let snapshot = derive_judgment_snapshot_for_companies(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        ["metadata-only"],
        &facts,
        Vec::new(),
    )
    .unwrap();
    let machine = snapshot.company("metadata-only").unwrap();

    assert_eq!(
        machine.reference_model_assessment().eligibility(),
        ReferenceModelEligibility::Confirmed
    );
    assert_eq!(machine.machine_stage().label(), "REFERENCE_MODEL");
}

#[test]
fn report_renders_localized_reference_model_matrix_without_calling_candidate_an_exemplar() {
    let facts = confirmed_reference_model_facts("reference");
    let judgment = derive_judgment_snapshot_for_companies(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        ["reference"],
        &facts,
        Vec::new(),
    )
    .unwrap();
    let mut input = RuntimeReportInput::from_date(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap());
    for fact in facts {
        input.add_fact(fact).unwrap();
    }
    input.set_judgment(judgment).unwrap();

    let chinese = render_report(&input).markdown().to_owned();
    assert!(chinese.contains("AI 时代范本验证"));
    assert!(chinese.contains("资格状态：已确认"));
    assert!(chinese.contains("组织重写：已具备"));
    assert!(chinese.contains("行业扩散：已具备"));
    assert!(!chinese.contains("候选范本"));

    let english = org_x::features::weekly_radar::runtime::report::render_report_in_language(
        &input,
        org_x::features::weekly_radar::runtime::report::ReportLanguage::English,
    );
    assert!(english
        .markdown()
        .contains("AI-era Reference Model Validation"));
    assert!(english.markdown().contains("Eligibility:Confirmed"));
    assert!(english
        .snapshot_json()
        .contains("reference_model_assessment"));
}

#[test]
fn report_does_not_render_internal_counter_review_as_company_observation() {
    let mut facts = confirmed_reference_model_facts("reference");
    facts.push(fact(
        "reference",
        "judgment.review.REFERENCE_MODEL.counter_evidence_review",
        "Counter-evidence review completed.",
        "https://example.test/review",
    ));
    let judgment = derive_judgment_snapshot_for_companies(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        ["reference"],
        &facts,
        Vec::new(),
    )
    .unwrap();
    let mut input = RuntimeReportInput::from_date(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap());
    for fact in facts {
        input.add_fact(fact).unwrap();
    }
    input.set_judgment(judgment).unwrap();

    let report = org_x::features::weekly_radar::runtime::report::render_report_in_language(
        &input,
        org_x::features::weekly_radar::runtime::report::ReportLanguage::English,
    );
    assert!(!report.markdown().contains(
        "Companies to Watch\n### reference\n- Other material:Counter-evidence review completed."
    ));
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
fn report_explains_machine_proofs_and_keeps_human_reference_separate() {
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
    for fact in facts {
        input.add_fact(fact).unwrap();
    }
    input.set_judgment(judgment).unwrap();

    let markdown = render_report(&input).markdown().to_owned();
    assert!(markdown.contains("workflow responsibility changed"));
    assert!(markdown.contains("https://source-a.example/workflow"));
    assert!(markdown.contains("quarterly_persistence"));
    assert!(markdown.contains("人的参考与系统不同。"));
    assert!(markdown.contains("系统判断"));
}

#[test]
fn report_does_not_render_machine_ranking_without_explicit_company_selection() {
    let mut facts = workflow_facts("acme");
    facts.extend(workflow_facts("beta"));
    let judgment = derive_judgment_snapshot(
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        &facts,
        Vec::new(),
    )
    .unwrap();
    let mut input = RuntimeReportInput::from_date(NaiveDate::from_ymd_opt(2026, 8, 20).unwrap());
    for fact in facts {
        input.add_fact(fact).unwrap();
    }
    input.set_judgment(judgment).unwrap();

    let markdown = render_report(&input).markdown().to_owned();
    assert!(!markdown.contains("同一阶段内的系统排序参考"));
    assert!(!markdown.contains("1. acme"));
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
