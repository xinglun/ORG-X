use super::*;

fn metrics() -> (
    RevenuePerEmployee,
    OperatingIncomePerEmployee,
    FreeCashFlowPerEmployee,
) {
    (
        RevenuePerEmployee::new("250000", "USD", "100").unwrap(),
        OperatingIncomePerEmployee::new("50000", "USD", "100").unwrap(),
        FreeCashFlowPerEmployee::new("40000", "USD", "100").unwrap(),
    )
}

fn growth() -> GrowthAndHeadcount {
    GrowthAndHeadcount::new(
        Some(GrowthRate::new("0.20").unwrap()),
        Some(GrowthRate::new("0.25").unwrap()),
        Some(GrowthRate::new("0.30").unwrap()),
        HeadcountChange::new("-0.05").unwrap(),
    )
    .unwrap()
}

#[test]
fn productivity_history_preserves_metrics_period_and_growth_facts() {
    let (revenue, operating_income, free_cash_flow) = metrics();
    let snapshot = ProductivitySnapshot::new(
        ProductivitySnapshotId::new("snapshot-1").unwrap(),
        "2026-Q2",
        Some(revenue.clone()),
        Some(operating_income.clone()),
        Some(free_cash_flow.clone()),
        growth(),
    )
    .unwrap();
    let mut history = ProductivityHistory::new(CompanyReference::new("company-1").unwrap());

    history.add_snapshot(snapshot.clone()).unwrap();

    assert_eq!(history.snapshots(), std::slice::from_ref(&snapshot));
    assert_eq!(history.snapshots()[0].period().as_str(), "2026-Q2");
    assert_eq!(
        history.snapshots()[0]
            .revenue_per_employee()
            .unwrap()
            .employee_count()
            .as_str(),
        "100"
    );
    assert_eq!(
        history.snapshots()[0]
            .operating_income_per_employee()
            .unwrap()
            .value()
            .as_str(),
        "50000"
    );
    assert_eq!(
        history.snapshots()[0]
            .free_cash_flow_per_employee()
            .unwrap()
            .unit()
            .as_str(),
        "USD"
    );
    assert_eq!(
        history.snapshots()[0]
            .growth_and_headcount()
            .headcount_change()
            .as_str(),
        "-0.05"
    );
}

#[test]
fn productivity_domain_rejects_blank_values_and_duplicate_snapshots() {
    assert!(matches!(
        CompanyReference::new(" "),
        Err(ProductivityDomainError::EmptyValue {
            field: "company reference"
        })
    ));
    assert!(matches!(
        RevenuePerEmployee::new("", "USD", "100"),
        Err(ProductivityDomainError::EmptyValue {
            field: "metric value"
        })
    ));
    assert!(matches!(
        GrowthRate::new(""),
        Err(ProductivityDomainError::EmptyValue {
            field: "growth rate"
        })
    ));

    let (_, _, _) = metrics();
    let snapshot = ProductivitySnapshot::new(
        ProductivitySnapshotId::new("snapshot").unwrap(),
        "2026-Q2",
        None,
        None,
        None,
        growth(),
    )
    .unwrap();
    let mut history = ProductivityHistory::new(CompanyReference::new("company").unwrap());
    history.add_snapshot(snapshot.clone()).unwrap();

    assert!(matches!(
        history.add_snapshot(snapshot),
        Err(ProductivityDomainError::DuplicateIdentity {
            entity: "productivity snapshot",
            ..
        })
    ));
}
