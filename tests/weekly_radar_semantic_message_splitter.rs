use org_x::features::weekly_radar::interface::semantic_message_splitter::{
    SemanticBoundary, SemanticMessageSplitter, SemanticSplitError, SemanticSplitLimits,
};

const RENDERED: &str = "*Weekly Radar — 2026-W33*\n\n## Important Structural Change\n- workflow transfer\n\n## Stage Transition\n- productivity breakout\n\n## Top5\n- **Acme** — complete card\n\n## Threshold Distance\n- **Acme** — Near\n\n## Rising\n- **Beta** — complete card\n\n## Dropped\n- **Gamma** — complete card\n\n## System Health\n- **Healthy** — current\n";

#[test]
fn rendered_sections_become_complete_semantic_chunks_without_reordering() {
    let split =
        SemanticMessageSplitter::split(RENDERED, SemanticSplitLimits::new(10_000, 100).unwrap())
            .unwrap();
    assert_eq!(split.chunks().len(), 5);
    assert_eq!(
        split.chunks()[0].boundary(),
        SemanticBoundary::ExecutiveSummary
    );
    assert_eq!(
        split.chunks()[1].boundary(),
        SemanticBoundary::ImportantTransition
    );
    assert_eq!(split.chunks()[2].boundary(), SemanticBoundary::Top5);
    assert_eq!(
        split.chunks()[3].boundary(),
        SemanticBoundary::RisingDropped
    );
    assert_eq!(split.chunks()[4].boundary(), SemanticBoundary::SystemHealth);
    assert!(split.chunks()[2]
        .markdown()
        .contains("**Acme** — complete card"));
}

#[test]
fn splitter_preserves_nested_markdown_and_code_fences() {
    let rendered = "*Weekly Radar — 2026-W33*\n\n## Top5\n- **Acme** — *card*\n  ```markdown\n  ## Rising\n  - **not a section**\n  ```\n";
    let split =
        SemanticMessageSplitter::split(rendered, SemanticSplitLimits::new(1_000, 50).unwrap())
            .unwrap();
    assert!(split.chunks()[1].markdown().contains("## Rising"));
    assert!(split.chunks()[1].markdown().contains("**not a section**"));
}

#[test]
fn splitter_rejects_zero_limits_unknown_sections_and_unclosed_fences() {
    assert_eq!(
        SemanticSplitLimits::new(0, 10),
        Err(SemanticSplitError::InvalidLimit {
            field: "max characters"
        })
    );
    assert!(matches!(
        SemanticMessageSplitter::split(
            "*title*\n\n## Future\n- x",
            SemanticSplitLimits::new(100, 10).unwrap()
        ),
        Err(SemanticSplitError::UnknownSection { .. })
    ));
    assert_eq!(
        SemanticMessageSplitter::split(
            "```markdown\n## Top5",
            SemanticSplitLimits::new(100, 10).unwrap()
        ),
        Err(SemanticSplitError::UnclosedCodeFence)
    );
}

#[test]
fn oversized_atomic_section_returns_no_partial_chunks() {
    let error =
        SemanticMessageSplitter::split(RENDERED, SemanticSplitLimits::new(20, 100).unwrap())
            .unwrap_err();
    assert!(matches!(
        error,
        SemanticSplitError::AtomicSectionTooLarge { .. }
    ));
}

#[test]
fn splitter_source_has_no_provider_or_domain_recomputation_boundary() {
    let source =
        include_str!("../src/features/weekly_radar/interface/semantic_message_splitter.rs");
    assert!(!source.contains("reqwest"));
    assert!(!source.contains("serde_json"));
    assert!(!source.contains("calculate_stage"));
    assert!(!source.contains("recalculate_ranking"));
}
