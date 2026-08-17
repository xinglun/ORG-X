use org_x::features::diffusion::domain::{
    BenchmarkObservation, CompanyReference, CompetitorImitation, DiffusionFactId, DiffusionProfile,
    DiffusionSignal, DiffusionSignalKind, IndustryDiffusion, JobTaxonomyChange,
};

fn company(value: &str) -> CompanyReference {
    CompanyReference::new(value).expect("company reference should be valid")
}

fn fact_id(value: &str) -> DiffusionFactId {
    DiffusionFactId::new(value).expect("fact identity should be valid")
}

#[test]
fn public_profile_keeps_explicit_diffusion_facts_without_inference() {
    let mut profile = DiffusionProfile::new(company("subject-co")).expect("profile is valid");
    profile
        .add_competitor_imitation(
            CompetitorImitation::new(
                fact_id("imitation-1"),
                company("subject-co"),
                company("imitator-a"),
                "workflow transfer",
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
                "role taxonomy changed",
                "2026-08-02",
            )
            .expect("taxonomy is valid"),
        )
        .expect("taxonomy is accepted");
    profile
        .add_benchmark(
            BenchmarkObservation::new(
                fact_id("benchmark-1"),
                company("subject-co"),
                "peer benchmark",
                "comparison retained",
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
                "diffusion observation retained",
                "2026-08-04",
            )
            .expect("industry fact is valid"),
        )
        .expect("industry fact is accepted");
    profile
        .add_signal(
            DiffusionSignal::new(
                fact_id("signal-1"),
                DiffusionSignalKind::WorkflowRedesign,
                "workflow redesign signal",
                "2026-08-05",
            )
            .expect("signal is valid"),
        )
        .expect("signal is accepted");

    assert_eq!(profile.competitor_imitations().len(), 1);
    assert_eq!(profile.job_taxonomy_changes().len(), 1);
    assert_eq!(profile.benchmarks().len(), 1);
    assert_eq!(profile.industry_diffusions().len(), 1);
    assert_eq!(
        profile.signals()[0].kind(),
        DiffusionSignalKind::WorkflowRedesign
    );
}
