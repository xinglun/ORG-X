use org_x::features::reporting::domain::{
    CompanyReference, ReportSection, ResearchCard, ResearchCardId, ResearchPacket, Top5,
};

fn card(identity: &str) -> ResearchCard {
    ResearchCard::new(
        ResearchCardId::new(identity).expect("card identity should be valid"),
        CompanyReference::new("alpha").expect("company should be valid"),
        "PRODUCTION_SYSTEM",
        "structural change",
        "supporting evidence",
        "counter evidence",
        "missing proof",
        "next step",
    )
    .expect("card should be valid")
}

#[test]
fn public_reporting_packet_is_a_read_only_grouping_boundary() {
    let mut top5 = Top5::new();
    top5.add(card("top-1")).expect("top card is accepted");
    let mut dropped = ReportSection::new();
    dropped
        .add(card("dropped-1"))
        .expect("dropped card is accepted");

    let packet = ResearchPacket::new(
        "No meaningful structural change this week.",
        top5,
        ReportSection::new(),
        ReportSection::new(),
        dropped,
    )
    .expect("packet should be valid");

    assert_eq!(packet.top5().cards().len(), 1);
    assert_eq!(packet.dropped().cards()[0].id().as_str(), "dropped-1");
    assert_eq!(
        packet.executive_summary().as_str(),
        "No meaningful structural change this week."
    );
}
