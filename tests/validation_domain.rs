use org_x::features::validation::application::validation_evaluator::{
    ValidationEvaluator, ValidationReadiness,
};
use org_x::features::validation::domain::{
    EvidenceReference, MetricObservation, PeerBaseline, SourceQuality, ValidationBaseline,
    ValidationDomainError, ValidationHorizon, ValidationObservation, ValidationRecord,
    ValidationSignal, ValidationStatus,
};

fn evidence(id: &str) -> EvidenceReference {
    EvidenceReference::new(id, format!("description-{id}")).expect("test evidence should be valid")
}

fn metric(name: &str, evidence_id: &str) -> MetricObservation {
    MetricObservation::new(
        name,
        "42",
        "opaque-units",
        SourceQuality::Primary,
        vec![evidence(evidence_id)],
    )
    .expect("test metric should be valid")
}

fn signal(note: &str, evidence_id: &str) -> ValidationSignal {
    ValidationSignal::new(
        ValidationStatus::Confirmed,
        note,
        vec![evidence(evidence_id)],
    )
    .expect("test signal should be valid")
}

fn baseline(company_id: &str) -> ValidationBaseline {
    ValidationBaseline::new(
        company_id,
        "STAGE_2_SUPPLIED",
        vec![evidence(&format!("{company_id}-baseline"))],
        vec!["hypothesis supplied at T0"],
        vec![evidence(&format!("{company_id}-counter"))],
        vec!["missing proof supplied at T0"],
        vec![PeerBaseline::new(
            "peer-group",
            metric("Revenue/Employee", &format!("{company_id}-peer")),
        )
        .expect("test peer baseline should be valid")],
    )
    .expect("test baseline should be valid")
}

fn observation(company_id: &str, horizon: ValidationHorizon) -> ValidationObservation {
    let prefix = format!("{company_id}-{}", horizon.as_str());
    ValidationObservation::new(
        horizon,
        format!("{prefix}-observed-at"),
        signal(
            "productivity divergence supplied",
            &format!("{prefix}-productivity"),
        ),
        signal("economic capture supplied", &format!("{prefix}-economic")),
        signal(
            "production continuity supplied",
            &format!("{prefix}-production"),
        ),
        signal(
            "competitor imitation supplied",
            &format!("{prefix}-imitation"),
        ),
        signal(
            "industry diffusion supplied",
            &format!("{prefix}-diffusion"),
        ),
        vec![metric("Revenue/Employee", &format!("{prefix}-metric"))],
    )
    .expect("test observation should be valid")
}

#[test]
fn record_retains_baseline_observations_and_reports_only_completeness() {
    let mut record = ValidationRecord::new(baseline("acme")).unwrap();
    assert_eq!(record.company_id(), "acme");
    assert_eq!(record.baseline().stage(), "STAGE_2_SUPPLIED");
    assert_eq!(record.missing_horizons(), ValidationHorizon::FOLLOW_UPS);
    assert_eq!(
        record.baseline().hypotheses(),
        &["hypothesis supplied at T0"]
    );
    assert_eq!(record.baseline().counter_evidence().len(), 1);
    assert_eq!(
        record.baseline().missing_proof(),
        &["missing proof supplied at T0"]
    );
    assert_eq!(
        record.baseline().peer_baseline()[0]
            .metric()
            .source_quality(),
        SourceQuality::Primary
    );

    record
        .add_observation(observation("acme", ValidationHorizon::SixMonths))
        .unwrap();
    let assessment = ValidationEvaluator::assess(&record);
    assert_eq!(assessment.company_id(), "acme");
    assert_eq!(
        assessment.missing_horizons(),
        &[
            ValidationHorizon::TwelveMonths,
            ValidationHorizon::TwentyFourMonths
        ]
    );
    assert_eq!(assessment.readiness(), ValidationReadiness::Incomplete);
    assert_eq!(record.observations()[0].metrics()[0].value(), "42");
    assert_eq!(record.observations()[0].metrics()[0].unit(), "opaque-units");
    assert_eq!(
        record.observations()[0].metrics()[0].source_quality(),
        SourceQuality::Primary
    );
    assert_eq!(
        record.observations()[0].productivity_divergence().note(),
        "productivity divergence supplied"
    );

    record
        .add_observation(observation("acme", ValidationHorizon::TwelveMonths))
        .unwrap();
    record
        .add_observation(observation("acme", ValidationHorizon::TwentyFourMonths))
        .unwrap();
    let complete = ValidationEvaluator::assess(&record);
    assert_eq!(complete.missing_horizons(), &[]);
    assert_eq!(complete.readiness(), ValidationReadiness::Complete);
}

#[test]
fn duplicate_horizon_is_rejected_without_mutating_record() {
    let mut record = ValidationRecord::new(baseline("acme")).unwrap();
    record
        .add_observation(observation("acme", ValidationHorizon::SixMonths))
        .unwrap();
    let before = record.observations().to_vec();

    assert_eq!(
        record.add_observation(observation("acme", ValidationHorizon::SixMonths)),
        Err(ValidationDomainError::DuplicateHorizon {
            horizon: ValidationHorizon::SixMonths,
        })
    );
    assert_eq!(record.observations(), before.as_slice());
}

#[test]
fn blank_values_and_duplicate_identities_are_rejected_at_the_boundary() {
    assert_eq!(
        EvidenceReference::new(" ", "description"),
        Err(ValidationDomainError::EmptyValue {
            field: "evidence id"
        })
    );
    assert_eq!(
        MetricObservation::new(
            "metric",
            "value",
            "unit",
            SourceQuality::Unknown,
            vec![evidence("same"), evidence("same")],
        ),
        Err(ValidationDomainError::DuplicateEvidenceId { id: "same".into() })
    );
}

#[test]
fn duplicate_metric_names_are_rejected_without_creating_an_observation() {
    let result = ValidationObservation::new(
        ValidationHorizon::SixMonths,
        "2026-01-01",
        signal("productivity", "productivity"),
        signal("economic", "economic"),
        signal("production", "production"),
        signal("imitation", "imitation"),
        signal("diffusion", "diffusion"),
        vec![metric("same", "metric-1"), metric("same", "metric-2")],
    );

    assert_eq!(
        result,
        Err(ValidationDomainError::DuplicateMetricName {
            name: "same".into(),
        })
    );
}

#[test]
fn evidence_overlap_between_baseline_and_follow_up_is_rejected_without_mutation() {
    let mut record = ValidationRecord::new(baseline("acme")).unwrap();
    let duplicate_baseline_reference = signal("reused evidence", "acme-baseline");
    let observation = ValidationObservation::new(
        ValidationHorizon::SixMonths,
        "2026-01-01",
        duplicate_baseline_reference,
        signal("economic", "overlap-economic"),
        signal("production", "overlap-production"),
        signal("imitation", "overlap-imitation"),
        signal("diffusion", "overlap-diffusion"),
        vec![metric("Revenue/Employee", "overlap-metric")],
    )
    .unwrap();
    let before = record.clone();

    assert_eq!(
        record.add_observation(observation),
        Err(ValidationDomainError::DuplicateEvidenceId {
            id: "acme-baseline".into(),
        })
    );
    assert_eq!(record, before);
}
