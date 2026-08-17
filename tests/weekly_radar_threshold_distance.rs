#[path = "../src/features/weekly_radar/domain/threshold_distance.rs"]
mod threshold_distance;

use threshold_distance::{
    CompanyReference, Distance, EvidenceId, StageLabel, ThresholdDistance,
    ThresholdDistanceDomainError,
};

fn company() -> CompanyReference {
    CompanyReference::new("acme").expect("company reference should be valid")
}

fn stage(value: &str) -> StageLabel {
    StageLabel::new(value).expect("stage label should be valid")
}

fn evidence(value: &str) -> EvidenceId {
    EvidenceId::new(value).expect("evidence identity should be valid")
}

#[test]
fn distance_labels_are_stable_and_complete() {
    assert_eq!(Distance::Far.label(), "FAR");
    assert_eq!(Distance::Developing.label(), "DEVELOPING");
    assert_eq!(Distance::Near.label(), "NEAR");
    assert_eq!(Distance::Candidate.label(), "CANDIDATE");
}

#[test]
fn threshold_distance_retains_supplied_values_and_order() {
    let threshold = ThresholdDistance::new(
        company(),
        stage("opaque-current"),
        stage("opaque-next"),
        vec![evidence("confirmed-1"), evidence("confirmed-2")],
        vec![evidence("missing-1"), evidence("missing-2")],
        Distance::Near,
    )
    .expect("supplied threshold distance should be accepted");

    assert_eq!(threshold.company().as_str(), "acme");
    assert_eq!(threshold.current_stage().as_str(), "opaque-current");
    assert_eq!(threshold.next_stage().as_str(), "opaque-next");
    assert_eq!(threshold.distance(), Distance::Near);
    assert_eq!(
        threshold
            .confirmed_evidence()
            .iter()
            .map(EvidenceId::as_str)
            .collect::<Vec<_>>(),
        ["confirmed-1", "confirmed-2"]
    );
    assert_eq!(
        threshold
            .missing_evidence()
            .iter()
            .map(EvidenceId::as_str)
            .collect::<Vec<_>>(),
        ["missing-1", "missing-2"]
    );
}

#[test]
fn supplied_distance_is_not_inferred_from_opaque_stages_or_evidence() {
    let threshold = ThresholdDistance::new(
        company(),
        stage("REFERENCE_MODEL"),
        stage("TOOL"),
        vec![evidence("confirmed")],
        vec![evidence("missing")],
        Distance::Far,
    )
    .expect("the upstream value should be retained even when labels suggest another relation");

    assert_eq!(threshold.distance(), Distance::Far);
}

#[test]
fn blank_values_and_empty_evidence_collections_are_rejected() {
    assert_eq!(
        CompanyReference::new("  "),
        Err(ThresholdDistanceDomainError::EmptyValue {
            field: "company reference"
        })
    );
    assert_eq!(
        StageLabel::new("\n"),
        Err(ThresholdDistanceDomainError::EmptyValue {
            field: "stage label"
        })
    );
    assert_eq!(
        EvidenceId::new(""),
        Err(ThresholdDistanceDomainError::EmptyValue {
            field: "evidence id"
        })
    );

    let error = ThresholdDistance::new(
        company(),
        stage("current"),
        stage("next"),
        Vec::new(),
        vec![evidence("missing")],
        Distance::Developing,
    )
    .expect_err("confirmed evidence must not be empty");
    assert_eq!(
        error,
        ThresholdDistanceDomainError::EmptyCollection {
            field: "confirmed evidence"
        }
    );
}

#[test]
fn duplicate_evidence_within_either_collection_is_rejected() {
    let duplicate_confirmed = ThresholdDistance::new(
        company(),
        stage("current"),
        stage("next"),
        vec![evidence("same"), evidence("same")],
        vec![evidence("missing")],
        Distance::Candidate,
    )
    .expect_err("duplicate confirmed evidence must be rejected");
    assert_eq!(
        duplicate_confirmed,
        ThresholdDistanceDomainError::DuplicateEvidence {
            collection: "confirmed evidence",
            id: "same".to_owned()
        }
    );

    let duplicate_missing = ThresholdDistance::new(
        company(),
        stage("current"),
        stage("next"),
        vec![evidence("confirmed")],
        vec![evidence("same"), evidence("same")],
        Distance::Candidate,
    )
    .expect_err("duplicate missing evidence must be rejected");
    assert_eq!(
        duplicate_missing,
        ThresholdDistanceDomainError::DuplicateEvidence {
            collection: "missing evidence",
            id: "same".to_owned()
        }
    );
}

#[test]
fn confirmed_and_missing_overlap_is_rejected() {
    let error = ThresholdDistance::new(
        company(),
        stage("current"),
        stage("next"),
        vec![evidence("shared"), evidence("confirmed")],
        vec![evidence("missing"), evidence("shared")],
        Distance::Near,
    )
    .expect_err("one evidence identity cannot be both confirmed and missing");

    assert_eq!(
        error,
        ThresholdDistanceDomainError::EvidenceOverlap {
            id: "shared".to_owned()
        }
    );
}
