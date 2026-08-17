use super::*;

fn company(value: &str) -> CompanyReference {
    CompanyReference::new(value).expect("company reference should be valid")
}

fn fact_id(value: &str) -> DiffusionFactId {
    DiffusionFactId::new(value).expect("fact identity should be valid")
}

#[test]
fn profile_preserves_imitation_taxonomy_benchmark_and_industry_order() {
    let mut profile = DiffusionProfile::new(company("subject-co")).expect("profile is valid");

    profile
        .add_competitor_imitation(
            CompetitorImitation::new(
                fact_id("imitation-1"),
                company("subject-co"),
                company("imitator-a"),
                "workflow redesign",
                "2026-08-01",
            )
            .expect("imitation is valid"),
        )
        .expect("imitation is accepted");
    profile
        .add_job_taxonomy_change(
            JobTaxonomyChange::new(
                fact_id("taxonomy-1"),
                company("imitator-a"),
                "agent supervisor",
                "new role created",
                "2026-08-02",
            )
            .expect("taxonomy change is valid"),
        )
        .expect("taxonomy change is accepted");
    profile
        .add_benchmark(
            BenchmarkObservation::new(
                fact_id("benchmark-1"),
                company("subject-co"),
                "peer productivity benchmark",
                "above peer baseline",
                "2026-Q2",
                "2026-08-03",
            )
            .expect("benchmark is valid"),
        )
        .expect("benchmark is accepted");
    profile
        .add_industry_diffusion(
            IndustryDiffusion::new(
                fact_id("industry-1"),
                "software",
                "workflow pattern observed across named peers",
                "2026-08-04",
            )
            .expect("industry fact is valid"),
        )
        .expect("industry fact is accepted");

    assert_eq!(profile.competitor_imitations().len(), 1);
    assert_eq!(profile.job_taxonomy_changes().len(), 1);
    assert_eq!(profile.benchmarks().len(), 1);
    assert_eq!(profile.industry_diffusions().len(), 1);
    assert_eq!(
        profile.competitor_imitations()[0]
            .imitator_company()
            .as_str(),
        "imitator-a"
    );
}

#[test]
fn profile_retains_signal_kind_without_assigning_stage_or_score() {
    let mut profile = DiffusionProfile::new(company("subject-co")).expect("profile is valid");
    let signal = DiffusionSignal::new(
        fact_id("signal-1"),
        DiffusionSignalKind::JobTaxonomy,
        "new job taxonomy recorded",
        "2026-08-05",
    )
    .expect("signal is valid");

    profile.add_signal(signal).expect("signal is accepted");

    assert_eq!(
        profile.signals()[0].kind(),
        DiffusionSignalKind::JobTaxonomy
    );
    assert_eq!(
        profile.signals()[0].description().as_str(),
        "new job taxonomy recorded"
    );
}

#[test]
fn profile_rejects_blank_values_and_duplicate_identities() {
    assert!(matches!(
        CompanyReference::new("   "),
        Err(DiffusionDomainError::EmptyValue { .. })
    ));

    let mut profile = DiffusionProfile::new(company("subject-co")).expect("profile is valid");
    let observation = IndustryDiffusion::new(
        fact_id("industry-1"),
        "software",
        "named peer observation",
        "2026-08-06",
    )
    .expect("industry fact is valid");
    profile
        .add_industry_diffusion(observation.clone())
        .expect("first fact is accepted");

    assert_eq!(
        profile.add_industry_diffusion(observation),
        Err(DiffusionDomainError::DuplicateIdentity {
            entity: "diffusion fact",
            id: "industry-1".to_owned(),
        })
    );
}
