use super::{EligibilityFacts, EligibilityPolicy, Exchange, InstrumentType};

#[test]
fn mvp_policy_evaluates_supplied_facts_without_external_state() {
    let policy = EligibilityPolicy::mvp();
    let facts = EligibilityFacts::new(InstrumentType::CommonEquity, Exchange::Nasdaq, true, true);

    assert!(policy.is_eligible(&facts));
}
