use org_x::features::productivity::domain::{
    CompanyReference, FreeCashFlowPerEmployee, GrowthAndHeadcount, GrowthRate, HeadcountChange,
    OperatingIncomePerEmployee, ProductivityDomainError, ProductivityHistory, ProductivitySnapshot,
    ProductivitySnapshotId, RevenuePerEmployee,
};

#[test]
fn public_productivity_api_retains_growth_and_headcount_without_calculation() {
    let growth = GrowthAndHeadcount::new(
        Some(GrowthRate::new("0.12").unwrap()),
        None,
        Some(GrowthRate::new("0.18").unwrap()),
        HeadcountChange::new("0.03").unwrap(),
    )
    .unwrap();
    let snapshot = ProductivitySnapshot::new(
        ProductivitySnapshotId::new("snapshot").unwrap(),
        "2026-Q1",
        Some(RevenuePerEmployee::new("200000", "USD", "50").unwrap()),
        Some(OperatingIncomePerEmployee::new("30000", "USD", "50").unwrap()),
        Some(FreeCashFlowPerEmployee::new("25000", "USD", "50").unwrap()),
        growth,
    )
    .unwrap();
    let mut history = ProductivityHistory::new(CompanyReference::new("company").unwrap());

    history.add_snapshot(snapshot).unwrap();

    assert_eq!(history.snapshots()[0].period().as_str(), "2026-Q1");
    assert_eq!(
        history.snapshots()[0]
            .growth_and_headcount()
            .revenue_growth()
            .unwrap()
            .as_str(),
        "0.12"
    );
    assert_eq!(
        history.snapshots()[0]
            .growth_and_headcount()
            .headcount_change()
            .as_str(),
        "0.03"
    );
}

#[test]
fn public_productivity_api_rejects_blank_denominator() {
    assert!(matches!(
        RevenuePerEmployee::new("200000", "USD", " "),
        Err(ProductivityDomainError::EmptyValue {
            field: "employee count"
        })
    ));
}
