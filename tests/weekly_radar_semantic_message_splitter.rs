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
fn splitter_accepts_localized_weekly_radar_headings() {
    let rendered = "## 本周摘要\n- 本周无变化\n\n## 系统参考判断\n### Acme\n- 系统判断：S2\n- 人的独立参考：S1\n\n## 系统状态\n- 数据正常\n";
    let split =
        SemanticMessageSplitter::split(rendered, SemanticSplitLimits::new(1_000, 20).unwrap())
            .unwrap();
    assert_eq!(split.chunks().len(), 3);
    assert_eq!(
        split.chunks()[0].boundary(),
        SemanticBoundary::ExecutiveSummary
    );
    assert_eq!(
        split.chunks()[1].boundary(),
        SemanticBoundary::JudgmentReference
    );
    assert_eq!(split.chunks()[2].boundary(), SemanticBoundary::SystemHealth);
}

#[test]
fn splitter_accepts_confirmed_information_as_part_of_the_reader_summary() {
    let rendered = "## 本周摘要\n- 本周有确认信息\n\n## 已确认信息\n### Acme\n- 证据：https://example.test/evidence\n\n## 系统状态\n- 数据正常\n";
    let split =
        SemanticMessageSplitter::split(rendered, SemanticSplitLimits::new(1_000, 20).unwrap())
            .expect("confirmed-information section should be a supported reader section");
    assert_eq!(split.chunks().len(), 2);
    assert_eq!(
        split.chunks()[0].boundary(),
        SemanticBoundary::ExecutiveSummary
    );
    assert!(split.chunks()[0].markdown().contains("## 已确认信息"));
    assert_eq!(split.chunks()[1].boundary(), SemanticBoundary::SystemHealth);
}

#[test]
fn splitter_accepts_validated_facts_and_structural_evidence_headings() {
    let rendered = "## Validated Facts\n### Acme\n- fact\n\n## Structural Evidence\n### Acme\n- workflow change\n\n## System Health\n- data available\n";
    let split =
        SemanticMessageSplitter::split(rendered, SemanticSplitLimits::new(1_000, 20).unwrap())
            .expect("new evidence sections should be supported reader sections");
    assert_eq!(split.chunks().len(), 3);
    assert_eq!(
        split.chunks()[0].boundary(),
        SemanticBoundary::ExecutiveSummary
    );
    assert_eq!(
        split.chunks()[1].boundary(),
        SemanticBoundary::ImportantTransition
    );
    assert_eq!(split.chunks()[2].boundary(), SemanticBoundary::SystemHealth);
}

#[test]
fn splitter_accepts_judgment_reference_heading_for_each_report_language() {
    for heading in [
        "系统参考判断",
        "システム参考判断",
        "System Reference Judgment",
    ] {
        let rendered = format!("## {heading}\n### Acme\n- complete reference\n");
        let split =
            SemanticMessageSplitter::split(&rendered, SemanticSplitLimits::new(1_000, 20).unwrap())
                .unwrap();
        assert_eq!(split.chunks().len(), 1);
        assert_eq!(
            split.chunks()[0].boundary(),
            SemanticBoundary::JudgmentReference
        );
        assert_eq!(split.chunks()[0].markdown(), rendered);
    }
}

#[test]
fn splitter_accepts_ai_era_reference_model_validation_heading_for_each_report_language() {
    for heading in [
        "AI 时代范本验证",
        "AI 時代の参照モデル検証",
        "AI-era Reference Model Validation",
    ] {
        let rendered = format!(
            "## 本周摘要\n- 本周无变化\n\n## {heading}\n### Acme\n- complete reference\n\n## 系统状态\n- 数据正常\n"
        );
        let split =
            SemanticMessageSplitter::split(&rendered, SemanticSplitLimits::new(1_000, 20).unwrap())
                .expect("AI-era reference model validation heading should be supported");
        assert_eq!(split.chunks().len(), 3);
        assert_eq!(
            split.chunks()[1].boundary(),
            SemanticBoundary::JudgmentReference
        );
        assert!(split.chunks()[1]
            .markdown()
            .contains(&format!("## {heading}")));
    }
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
