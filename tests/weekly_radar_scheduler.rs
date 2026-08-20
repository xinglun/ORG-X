use std::fs;
use std::path::Path;

use org_x::features::weekly_radar::application::weekly_scheduler::{
    ScheduleDecision, Weekday, WeeklySchedule, WeeklyScheduler,
};

#[test]
fn public_scheduler_uses_monday_by_default() {
    let scheduler = WeeklyScheduler::default();

    assert_eq!(scheduler.schedule().day_of_week(), Weekday::Monday);
    assert!(matches!(
        scheduler.evaluate(Weekday::Monday),
        ScheduleDecision::Due { .. }
    ));
    assert!(!scheduler.should_run(Weekday::Sunday));
}

#[test]
fn public_scheduler_honors_configured_day_of_week() {
    let scheduler = WeeklyScheduler::new(WeeklySchedule::new(Weekday::Thursday));

    assert_eq!(scheduler.schedule().day_of_week(), Weekday::Thursday);
    assert!(scheduler.should_run(Weekday::Thursday));
    assert!(!scheduler.should_run(Weekday::Friday));
}

#[test]
fn scheduler_decision_is_a_trigger_only_and_domain_has_no_scheduler_reference() {
    let decision = WeeklyScheduler::default().evaluate(Weekday::Saturday);

    assert_eq!(decision.scheduled_day(), Weekday::Monday);
    assert_eq!(decision.observed_day(), Weekday::Saturday);
    assert!(!decision.is_due());

    let domain_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/features/weekly_radar/domain");
    for entry in fs::read_dir(domain_root).expect("weekly radar domain should be readable") {
        let path = entry.expect("domain entry should be readable").path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path).expect("domain source should be readable");
            assert!(
                !source.to_ascii_lowercase().contains("scheduler"),
                "{} must not depend on scheduler terminology",
                path.display()
            );
        }
    }
}
