use super::{SemanticBoundary, SemanticMessageSplitter, SemanticSplitError, SemanticSplitLimits};

fn limits(characters: usize, lines: usize) -> SemanticSplitLimits {
    SemanticSplitLimits::new(characters, lines).unwrap()
}

const FIXTURE: &str = "*Weekly Radar — 2026-W33*\n\n## Important Structural Change\n- workflow\n\n## Stage Transition\n- breakout\n\n## Top5\n- **Acme** — card\n\n## Threshold Distance\n- **Acme** — near\n\n## Rising\n- **Beta** — rising\n\n## Dropped\n- **Gamma** — dropped\n\n## System Health\n- **Healthy** — current\n";

#[test]
fn module_local_test_maps_sections_in_source_order() {
    let result = SemanticMessageSplitter::split(FIXTURE, limits(10_000, 100)).unwrap();
    assert_eq!(
        result
            .chunks()
            .iter()
            .map(|chunk| chunk.boundary())
            .collect::<Vec<_>>(),
        vec![
            SemanticBoundary::ExecutiveSummary,
            SemanticBoundary::ImportantTransition,
            SemanticBoundary::Top5,
            SemanticBoundary::RisingDropped,
            SemanticBoundary::SystemHealth,
        ]
    );
    assert!(result.chunks()[1]
        .markdown()
        .contains("## Stage Transition"));
    assert!(result.chunks()[2]
        .markdown()
        .contains("## Threshold Distance"));
}

#[test]
fn module_local_test_protects_fenced_headings_and_atomic_sections() {
    let source =
        "*Weekly Radar — 2026-W33*\n\n## Top5\n```markdown\n## Rising\n- **inside**\n```\n";
    let result = SemanticMessageSplitter::split(source, limits(10_000, 100)).unwrap();
    assert_eq!(result.chunks().len(), 2);
    assert!(result.chunks()[1].markdown().contains("## Rising"));

    let error = SemanticMessageSplitter::split(FIXTURE, limits(20, 100)).unwrap_err();
    assert!(matches!(
        error,
        SemanticSplitError::AtomicSectionTooLarge { .. }
    ));
}

#[test]
fn module_local_test_packs_same_boundary_only_when_sections_fit() {
    let source = "*Weekly Radar — 2026-W33*\n\n## Important Structural Change\n- workflow\n\n## Stage Transition\n- breakout\n";
    let result = SemanticMessageSplitter::split(source, limits(60, 100)).unwrap();
    assert_eq!(result.chunks().len(), 3);
    assert_eq!(
        result.chunks()[1].boundary(),
        SemanticBoundary::ImportantTransition
    );
    assert_eq!(
        result.chunks()[2].boundary(),
        SemanticBoundary::ImportantTransition
    );
    let joined = result
        .chunks()
        .iter()
        .map(|chunk| chunk.markdown())
        .collect::<String>();
    assert_eq!(joined, source);
}

#[test]
fn module_local_test_rejects_invalid_rendered_input() {
    assert!(matches!(
        SemanticMessageSplitter::split("", limits(100, 10)),
        Err(SemanticSplitError::EmptyMessage)
    ));
    assert!(matches!(
        SemanticMessageSplitter::split("## Unknown\n- value", limits(100, 10)),
        Err(SemanticSplitError::UnknownSection { .. })
    ));
    assert!(matches!(
        SemanticMessageSplitter::split("```\n## Top5", limits(100, 10)),
        Err(SemanticSplitError::UnclosedCodeFence)
    ));
}

#[test]
fn module_local_test_accepts_localized_report_headings() {
    let source = "## 本周摘要\n- 本周无变化\n\n## 系统参考判断\n### Acme\n- 系统判断：S2\n- 人的独立参考：S1\n\n## システム状態\n- データ正常\n";
    let result = SemanticMessageSplitter::split(source, limits(1_000, 20)).unwrap();

    assert_eq!(result.chunks().len(), 3);
    assert_eq!(
        result.chunks()[0].boundary(),
        SemanticBoundary::ExecutiveSummary
    );
    assert_eq!(
        result.chunks()[1].boundary(),
        SemanticBoundary::JudgmentReference
    );
    assert_eq!(
        result.chunks()[2].boundary(),
        SemanticBoundary::SystemHealth
    );
}
