use super::*;

fn period() -> PeriodId {
    PeriodId::new("2026-W33").expect("period should be valid")
}

fn item(id: &str, markdown: &str) -> SummaryItem {
    SummaryItem::new(ItemId::new(id).expect("item id should be valid"), markdown)
        .expect("item should be valid")
}

fn card(id: &str, company: &str, markdown: &str) -> CompanyCard {
    CompanyCard::new(
        ItemId::new(id).expect("card id should be valid"),
        CompanyReference::new(company).expect("company should be valid"),
        markdown,
    )
    .expect("card should be valid")
}

fn limits() -> TelegramRenderLimits {
    TelegramRenderLimits::new(4_096, 40, 5, 20).expect("limits should be valid")
}

#[test]
fn module_local_test_preserves_explicit_values_and_order() {
    let input = TelegramSummaryInput::new(
        period(),
        vec![item("important-1", "first"), item("important-2", "second")],
        vec![item("transition-1", "transition")],
        vec![card("top-1", "Acme", "top card")],
        Vec::new(),
        vec![card("rising-1", "Beta", "rising card")],
        Vec::new(),
        Some(SystemHealthSummary::new("HEALTHY", "explicit health").unwrap()),
        None,
    )
    .expect("explicit input should be valid");

    assert_eq!(input.important_structural()[0].markdown().as_str(), "first");
    assert_eq!(
        input.important_structural()[1].markdown().as_str(),
        "second"
    );
    assert_eq!(input.top5()[0].company().as_str(), "Acme");
    assert_eq!(input.system_health().unwrap().status().as_str(), "HEALTHY");

    let message = TelegramRenderer::render(&input, limits()).expect("input should render");
    assert!(message.as_str().contains("top card"));
    assert!(message.as_str().contains("rising card"));
    assert_eq!(message.company_card_count(), 2);
}

#[test]
fn module_local_test_rejects_limits_and_preserves_atomic_failure() {
    assert_eq!(
        TelegramRenderLimits::new(0, 40, 5, 20),
        Err(TelegramRenderError::InvalidLimit {
            field: "max characters"
        })
    );

    let input = TelegramSummaryInput::new(
        period(),
        vec![item("one", "one"), item("two", "two")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        None,
    )
    .expect("explicit items should be valid");
    assert_eq!(
        TelegramRenderer::render(&input, TelegramRenderLimits::new(4_096, 40, 1, 20).unwrap(),),
        Err(TelegramRenderError::ItemLimitExceeded {
            section: "Important Structural Change",
            limit: 1,
            actual: 2,
        })
    );
}

#[test]
fn module_local_test_rejects_duplicate_identity_and_period_mismatch() {
    let duplicate = TelegramSummaryInput::new(
        period(),
        vec![item("same", "first")],
        vec![item("same", "second")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        None,
    )
    .expect_err("duplicate identities should be rejected");
    assert_eq!(
        duplicate,
        TelegramRenderError::DuplicateIdentity {
            id: "same".to_owned()
        }
    );

    let mismatch = TelegramSummaryInput::new(
        period(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        Some(NoChangeSummary::new(PeriodId::new("2026-W34").unwrap(), "no change").unwrap()),
    )
    .expect_err("period mismatch should be rejected");
    assert_eq!(
        mismatch,
        TelegramRenderError::PeriodMismatch {
            expected: "2026-W33".to_owned(),
            actual: "2026-W34".to_owned(),
        }
    );
}
