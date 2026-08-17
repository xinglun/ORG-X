use org_x::features::universe::domain::{
    Company, CompanyId, EligibilityFacts, EligibilityPolicy, Exchange, IndexMembership,
    InstrumentType, Listing, ListingId, Security, SecurityId, SnapshotId, UniverseDomainError,
    UniverseIndex, UniverseSnapshot,
};

fn company(id: &str, name: &str) -> Company {
    Company::new(CompanyId::new(id).unwrap(), name).unwrap()
}

fn security(id: &str, company_id: &str, instrument: InstrumentType) -> Security {
    Security::new(
        SecurityId::new(id).unwrap(),
        CompanyId::new(company_id).unwrap(),
        instrument,
    )
    .unwrap()
}

fn listing(id: &str, security_id: &str, exchange: Exchange, ticker: &str, active: bool) -> Listing {
    Listing::new(
        ListingId::new(id).unwrap(),
        SecurityId::new(security_id).unwrap(),
        exchange,
        ticker,
        active,
    )
    .unwrap()
}

fn snapshot(
    companies: Vec<Company>,
    securities: Vec<Security>,
    listings: Vec<Listing>,
    memberships: Vec<IndexMembership>,
) -> UniverseSnapshot {
    UniverseSnapshot::new(
        SnapshotId::new("snapshot-2026-08-17").unwrap(),
        companies,
        securities,
        listings,
        memberships,
    )
    .unwrap()
}

#[test]
fn rejects_empty_identity_and_display_values() {
    assert!(matches!(
        CompanyId::new(""),
        Err(UniverseDomainError::EmptyValue { .. })
    ));
    assert!(Company::new(CompanyId::new("company-acme").unwrap(), " ").is_err());
    assert!(Listing::new(
        ListingId::new("listing-acme").unwrap(),
        SecurityId::new("security-acme").unwrap(),
        Exchange::Nasdaq,
        "",
        true,
    )
    .is_err());
}

#[test]
fn admits_active_common_equity_with_supported_index_membership() {
    let snapshot = snapshot(
        vec![company("company-acme", "Acme Corp")],
        vec![security(
            "security-acme",
            "company-acme",
            InstrumentType::CommonEquity,
        )],
        vec![listing(
            "listing-acme",
            "security-acme",
            Exchange::Nasdaq,
            "ACME",
            true,
        )],
        vec![IndexMembership::new(
            SecurityId::new("security-acme").unwrap(),
            UniverseIndex::Sp500,
        )],
    );

    assert_eq!(
        snapshot.eligible_security_ids(&EligibilityPolicy::mvp()),
        vec![SecurityId::new("security-acme").unwrap()]
    );
}

#[test]
fn policy_evaluates_only_the_facts_it_receives() {
    let policy = EligibilityPolicy::mvp();
    let eligible = EligibilityFacts::new(InstrumentType::CommonEquity, Exchange::Nyse, true, true);
    let rejected = EligibilityFacts::new(InstrumentType::CommonEquity, Exchange::Nyse, true, false);

    assert!(policy.is_eligible(&eligible));
    assert!(!policy.is_eligible(&rejected));
}

#[test]
fn rejects_inactive_unsupported_non_common_and_unindexed_listings() {
    let inactive = snapshot(
        vec![company("company-inactive", "Inactive Corp")],
        vec![security(
            "security-inactive",
            "company-inactive",
            InstrumentType::CommonEquity,
        )],
        vec![listing(
            "listing-inactive",
            "security-inactive",
            Exchange::Nyse,
            "INAC",
            false,
        )],
        vec![IndexMembership::new(
            SecurityId::new("security-inactive").unwrap(),
            UniverseIndex::Nasdaq100,
        )],
    );
    assert!(inactive
        .eligible_security_ids(&EligibilityPolicy::mvp())
        .is_empty());

    let unsupported_exchange = snapshot(
        vec![company("company-foreign", "Foreign Corp")],
        vec![security(
            "security-foreign",
            "company-foreign",
            InstrumentType::CommonEquity,
        )],
        vec![listing(
            "listing-foreign",
            "security-foreign",
            Exchange::Other("LSE".to_owned()),
            "FOR",
            true,
        )],
        vec![IndexMembership::new(
            SecurityId::new("security-foreign").unwrap(),
            UniverseIndex::Sp500,
        )],
    );
    assert!(unsupported_exchange
        .eligible_security_ids(&EligibilityPolicy::mvp())
        .is_empty());

    let preferred = snapshot(
        vec![company("company-preferred", "Preferred Corp")],
        vec![security(
            "security-preferred",
            "company-preferred",
            InstrumentType::PreferredEquity,
        )],
        vec![listing(
            "listing-preferred",
            "security-preferred",
            Exchange::Nyse,
            "PREF",
            true,
        )],
        vec![IndexMembership::new(
            SecurityId::new("security-preferred").unwrap(),
            UniverseIndex::Nasdaq100,
        )],
    );
    assert!(preferred
        .eligible_security_ids(&EligibilityPolicy::mvp())
        .is_empty());

    let unindexed = snapshot(
        vec![company("company-unindexed", "Unindexed Corp")],
        vec![security(
            "security-unindexed",
            "company-unindexed",
            InstrumentType::CommonEquity,
        )],
        vec![listing(
            "listing-unindexed",
            "security-unindexed",
            Exchange::Nasdaq,
            "NONE",
            true,
        )],
        vec![],
    );
    assert!(unindexed
        .eligible_security_ids(&EligibilityPolicy::mvp())
        .is_empty());
}

#[test]
fn deduplicates_index_membership_and_returns_stable_security_order() {
    let snapshot = snapshot(
        vec![
            company("company-b", "Beta Corp"),
            company("company-a", "Alpha Corp"),
        ],
        vec![
            security("security-b", "company-b", InstrumentType::CommonEquity),
            security("security-a", "company-a", InstrumentType::CommonEquity),
        ],
        vec![
            listing("listing-b", "security-b", Exchange::Nyse, "BETA", true),
            listing("listing-a", "security-a", Exchange::Nasdaq, "ALFA", true),
        ],
        vec![
            IndexMembership::new(
                SecurityId::new("security-b").unwrap(),
                UniverseIndex::Nasdaq100,
            ),
            IndexMembership::new(SecurityId::new("security-a").unwrap(), UniverseIndex::Sp500),
            IndexMembership::new(
                SecurityId::new("security-a").unwrap(),
                UniverseIndex::Nasdaq100,
            ),
        ],
    );

    assert_eq!(
        snapshot.eligible_security_ids(&EligibilityPolicy::mvp()),
        vec![
            SecurityId::new("security-a").unwrap(),
            SecurityId::new("security-b").unwrap(),
        ]
    );
}

#[test]
fn rejects_snapshot_references_to_missing_entities() {
    let result = UniverseSnapshot::new(
        SnapshotId::new("snapshot-invalid").unwrap(),
        vec![company("company-acme", "Acme Corp")],
        vec![security(
            "security-acme",
            "company-missing",
            InstrumentType::CommonEquity,
        )],
        vec![],
        vec![],
    );

    assert!(matches!(
        result,
        Err(UniverseDomainError::UnknownCompany { .. })
    ));
}
