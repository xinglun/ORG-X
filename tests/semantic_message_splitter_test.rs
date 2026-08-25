#[path = "weekly_radar_semantic_message_splitter.rs"]
mod weekly_radar_semantic_message_splitter;

use org_x::features::weekly_radar::interface::semantic_message_splitter::{
    SemanticBoundary, SemanticMessageSplitter, SemanticSplitLimits,
};

#[test]
fn splitter_keeps_confirmed_information_in_the_reader_summary() {
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
fn splitter_accepts_structural_evidence_as_an_important_transition() {
    let rendered = "## 结构性证据\n### Acme\n- workflow change\n";
    let split =
        SemanticMessageSplitter::split(rendered, SemanticSplitLimits::new(1_000, 20).unwrap())
            .expect("structural evidence should be a supported reader section");
    assert_eq!(split.chunks().len(), 1);
    assert_eq!(
        split.chunks()[0].boundary(),
        SemanticBoundary::ImportantTransition
    );
}
