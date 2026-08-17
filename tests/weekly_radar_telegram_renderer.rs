#[path = "../src/features/weekly_radar/interface/telegram_renderer.rs"]
mod telegram_renderer;

use telegram_renderer::{
    CompanyCard, CompanyReference, ItemId, NoChangeSummary, PeriodId, SummaryItem,
    SystemHealthSummary, TelegramRenderError, TelegramRenderLimits, TelegramRenderer,
    TelegramSummaryInput,
};

fn period() -> PeriodId {
    PeriodId::new("2026-W33").expect("period should be valid")
}

fn item(id: &str, markdown: &str) -> SummaryItem {
    SummaryItem::new(ItemId::new(id).expect("item id should be valid"), markdown)
        .expect("summary item should be valid")
}

fn card(id: &str, company: &str, markdown: &str) -> CompanyCard {
    CompanyCard::new(
        ItemId::new(id).expect("card id should be valid"),
        CompanyReference::new(company).expect("company should be valid"),
        markdown,
    )
    .expect("company card should be valid")
}

fn limits() -> TelegramRenderLimits {
    TelegramRenderLimits::new(4_096, 40, 5, 20).expect("limits should be valid")
}

fn changed_input() -> TelegramSummaryInput {
    TelegramSummaryInput::new(
        period(),
        vec![item("isc-1", "**workflow proof** expanded")],
        vec![item("transition-1", "`WORKFLOW` → `PRODUCTION_SYSTEM`")],
        vec![card("top-1", "Acme", "*Top5 complete card with context*")],
        vec![card(
            "distance-1",
            "Beta",
            "Distance: `Near`; proof remains explicit",
        )],
        vec![card(
            "rising-1",
            "Gamma",
            "Rising: supporting evidence strengthened",
        )],
        vec![card(
            "dropped-1",
            "Delta",
            "Dropped: counter evidence invalidated prior proof",
        )],
        Some(
            SystemHealthSummary::new("DEGRADED", "coverage 3/5; source alpha is aging")
                .expect("health should be valid"),
        ),
        None,
    )
    .expect("changed input should be valid")
}

#[test]
fn renders_all_explicit_sections_in_priority_order_without_recomputing_facts() {
    let input = changed_input();
    assert_eq!(input.period().as_str(), "2026-W33");
    assert_eq!(
        input.stage_transitions()[0].markdown().as_str(),
        "`WORKFLOW` → `PRODUCTION_SYSTEM`"
    );
    assert_eq!(
        input.threshold_distances()[0].markdown().as_str(),
        "Distance: `Near`; proof remains explicit"
    );
    assert_eq!(
        input.rising()[0].markdown().as_str(),
        "Rising: supporting evidence strengthened"
    );
    assert_eq!(
        input.dropped()[0].markdown().as_str(),
        "Dropped: counter evidence invalidated prior proof"
    );
    assert_eq!(
        input.top5()[0].markdown().as_str(),
        "*Top5 complete card with context*"
    );

    let message =
        TelegramRenderer::render(&input, limits()).expect("explicit sections should render");
    let markdown = message.as_str();

    for fragment in [
        "**workflow proof** expanded",
        "`WORKFLOW` → `PRODUCTION_SYSTEM`",
        "*Top5 complete card with context*",
        "Distance: `Near`; proof remains explicit",
        "Rising: supporting evidence strengthened",
        "Dropped: counter evidence invalidated prior proof",
        "coverage 3/5; source alpha is aging",
    ] {
        assert!(markdown.contains(fragment), "missing fragment: {fragment}");
    }

    let markers = [
        "## Important Structural Change",
        "## Stage Transition",
        "## Top5",
        "## Threshold Distance",
        "## Rising",
        "## Dropped",
        "## System Health",
    ];
    let positions: Vec<_> = markers
        .iter()
        .map(|marker| markdown.find(marker).expect("section marker should exist"))
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(message.company_card_count(), 4);
    assert_eq!(message.character_count(), markdown.chars().count());
    assert_eq!(message.line_count(), markdown.lines().count());
}

#[test]
fn renders_explicit_no_change_without_inventing_change_sections() {
    let no_change = NoChangeSummary::new(period(), "No meaningful structural change this week.")
        .expect("no-change fact should be valid");
    assert_eq!(no_change.period().as_str(), "2026-W33");
    let input = TelegramSummaryInput::new(
        period(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        Some(no_change),
    )
    .expect("explicit no-change input should be valid");

    let markdown = TelegramRenderer::render(&input, limits())
        .expect("explicit no-change input should render")
        .as_str()
        .to_owned();
    assert!(markdown.contains("NO_CHANGE"));
    assert!(markdown.contains("No meaningful structural change this week."));
    assert!(!markdown.contains("## Top5"));
    assert!(!markdown.contains("## Rising"));
    assert!(!markdown.contains("## Dropped"));
}

#[test]
fn rejects_invalid_change_state_and_duplicate_identity() {
    let missing = TelegramSummaryInput::new(
        period(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        None,
    )
    .expect_err("empty input without explicit no-change must be rejected");
    assert_eq!(missing, TelegramRenderError::MissingChangeState);

    let contradictory = TelegramSummaryInput::new(
        period(),
        vec![item("change-1", "change")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        Some(NoChangeSummary::new(period(), "no change").expect("no-change fact should be valid")),
    )
    .expect_err("No Change cannot coexist with change items");
    assert_eq!(contradictory, TelegramRenderError::ConflictingNoChange);

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
    .expect_err("duplicate section identity must be rejected");
    assert_eq!(
        duplicate,
        TelegramRenderError::DuplicateIdentity {
            id: "same".to_owned()
        }
    );
}

#[test]
fn rejects_limits_without_truncating_markdown_or_company_cards() {
    let many_items = TelegramSummaryInput::new(
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
    .expect("multiple explicit items should be valid");
    assert_eq!(
        TelegramRenderer::render(
            &many_items,
            TelegramRenderLimits::new(4_096, 40, 1, 20).expect("limits should be valid"),
        ),
        Err(TelegramRenderError::ItemLimitExceeded {
            section: "Important Structural Change",
            limit: 1,
            actual: 2,
        })
    );

    let input = changed_input();
    assert_eq!(
        TelegramRenderer::render(
            &input,
            TelegramRenderLimits::new(4_096, 40, 5, 3).expect("limits should be valid"),
        ),
        Err(TelegramRenderError::CompanyCardLimitExceeded {
            limit: 3,
            actual: 4,
        })
    );

    let line_error = TelegramRenderer::render(
        &input,
        TelegramRenderLimits::new(4_096, 1, 5, 20).expect("limits should be valid"),
    )
    .expect_err("complete Markdown must fail atomically when line-limited");
    match line_error {
        TelegramRenderError::LineLimitExceeded { limit, actual } => {
            assert_eq!(limit, 1);
            assert!(actual > limit);
        }
        other => panic!("expected atomic line-limit error, got {other:?}"),
    }

    let long_card = TelegramSummaryInput::new(
        period(),
        Vec::new(),
        Vec::new(),
        vec![card("long", "Acme", "**full card**\nwith complete context")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        None,
    )
    .expect("long card input should be valid");
    let error = TelegramRenderer::render(
        &long_card,
        TelegramRenderLimits::new(12, 40, 5, 5).expect("limits should be valid"),
    )
    .expect_err("full card must fail atomically when message is too long");
    match error {
        TelegramRenderError::MessageTooLong { limit, actual } => {
            assert_eq!(limit, 12);
            assert!(actual > limit);
        }
        other => panic!("expected atomic message-length error, got {other:?}"),
    }
}

#[test]
fn rejects_blank_values_period_mismatch_and_source_provider_coupling() {
    assert!(matches!(
        ItemId::new("  "),
        Err(TelegramRenderError::EmptyValue { field: "item id" })
    ));
    assert!(matches!(
        CompanyReference::new("\n"),
        Err(TelegramRenderError::EmptyValue {
            field: "company reference"
        })
    ));
    assert!(matches!(
        SummaryItem::new(ItemId::new("id").unwrap(), "  "),
        Err(TelegramRenderError::EmptyValue {
            field: "markdown fragment"
        })
    ));

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
    .expect_err("No Change period must match the input period");
    assert_eq!(
        mismatch,
        TelegramRenderError::PeriodMismatch {
            expected: "2026-W33".to_owned(),
            actual: "2026-W34".to_owned(),
        }
    );

    let source = include_str!("../src/features/weekly_radar/interface/telegram_renderer.rs");
    for forbidden in [
        "use crate",
        "use org_x",
        "reqwest",
        "tokio",
        "ORGX_TELEGRAM",
    ] {
        assert!(
            !source.contains(forbidden),
            "renderer must not contain provider coupling: {forbidden}"
        );
    }
}
