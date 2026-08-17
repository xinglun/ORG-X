use super::*;

fn card(identity: &str) -> ResearchCard {
    ResearchCard::new(
        ResearchCardId::new(identity).expect("card identity should be valid"),
        CompanyReference::new("alpha").expect("company should be valid"),
        "WORKFLOW",
        "headline",
        "supporting evidence",
        "counter evidence",
        "missing proof",
        "next research step",
    )
    .expect("card should be valid")
}

#[test]
fn reporting_sections_preserve_card_facts_and_order() {
    let mut rising = ReportSection::new();
    rising
        .add(card("rising-1"))
        .expect("first card is accepted");
    rising
        .add(card("rising-2"))
        .expect("second card is accepted");

    assert_eq!(rising.cards().len(), 2);
    assert_eq!(rising.cards()[0].id().as_str(), "rising-1");
    assert_eq!(rising.cards()[0].stage().as_str(), "WORKFLOW");
    assert_eq!(
        rising.cards()[0].counter_evidence().as_str(),
        "counter evidence"
    );
}

#[test]
fn top5_rejects_the_sixth_card_and_duplicate_identity() {
    let mut top5 = Top5::new();
    for index in 1..=5 {
        top5.add(card(&format!("top-{index}")))
            .expect("top five cards are accepted");
    }

    assert_eq!(
        top5.add(card("top-6")),
        Err(ReportingDomainError::Top5LimitExceeded { limit: 5 })
    );
    assert_eq!(
        top5.add(card("top-1")),
        Err(ReportingDomainError::DuplicateIdentity {
            entity: "research card",
            id: "top-1".to_owned(),
        })
    );
}

#[test]
fn packet_groups_sections_without_recomputing_membership() {
    let mut top5 = Top5::new();
    top5.add(card("top-1")).expect("top card is accepted");
    let mut rising = ReportSection::new();
    rising
        .add(card("rising-1"))
        .expect("rising card is accepted");

    let packet = ResearchPacket::new(
        "weekly research packet",
        top5,
        rising,
        ReportSection::new(),
        ReportSection::new(),
    )
    .expect("packet should be valid");

    assert_eq!(packet.top5().cards()[0].id().as_str(), "top-1");
    assert_eq!(packet.rising().cards()[0].id().as_str(), "rising-1");
    assert!(packet.watch().cards().is_empty());
    assert_eq!(
        packet.executive_summary().as_str(),
        "weekly research packet"
    );
}

#[test]
fn reporting_rejects_blank_card_fields() {
    assert!(matches!(
        ResearchCardId::new("  "),
        Err(ReportingDomainError::EmptyValue { .. })
    ));
}
