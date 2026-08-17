use super::{MarkdownRenderError, RankChange, StageHistoryEntry};

#[test]
fn stage_history_rejects_blank_required_values() {
    assert_eq!(
        StageHistoryEntry::new(
            " ",
            "2026-W33",
            "Acme",
            "WORKFLOW",
            "PRODUCTION_SYSTEM",
            "fact",
        ),
        Err(MarkdownRenderError::EmptyValue {
            entity: "stage history",
            field: "id",
        })
    );
}

#[test]
fn rank_change_retains_optional_positions_and_fact() {
    let change = RankChange::new(
        "rank-1",
        "2026-W33",
        "Acme",
        Some(4),
        None,
        "left supplied Top5",
    )
    .expect("rank change should validate");

    assert_eq!(change.previous_rank(), Some(4));
    assert_eq!(change.current_rank(), None);
    assert_eq!(change.fact(), "left supplied Top5");
}
