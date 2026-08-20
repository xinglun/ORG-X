use super::{ScheduleDecision, Weekday, WeeklySchedule, WeeklyScheduler};

#[test]
fn default_schedule_runs_on_monday_and_not_on_sunday() {
    let scheduler = WeeklyScheduler::default();

    assert_eq!(scheduler.schedule().day_of_week(), Weekday::Monday);
    assert_eq!(
        scheduler.evaluate(Weekday::Monday),
        ScheduleDecision::Due {
            scheduled_day: Weekday::Monday,
            observed_day: Weekday::Monday,
        }
    );
    assert_eq!(
        scheduler.evaluate(Weekday::Sunday),
        ScheduleDecision::NotDue {
            scheduled_day: Weekday::Monday,
            observed_day: Weekday::Sunday,
        }
    );
}

#[test]
fn configured_day_is_the_only_due_day() {
    for configured_day in Weekday::ALL {
        let scheduler = WeeklyScheduler::new(WeeklySchedule::new(configured_day));

        for observed_day in Weekday::ALL {
            let expected = if observed_day == configured_day {
                ScheduleDecision::Due {
                    scheduled_day: configured_day,
                    observed_day,
                }
            } else {
                ScheduleDecision::NotDue {
                    scheduled_day: configured_day,
                    observed_day,
                }
            };

            assert_eq!(scheduler.evaluate(observed_day), expected);
        }
    }
}

#[test]
fn decision_retains_weekdays_and_has_stable_display() {
    let scheduler = WeeklyScheduler::new(WeeklySchedule::new(Weekday::Friday));
    let decision = scheduler.evaluate(Weekday::Monday);

    assert!(!decision.is_due());
    assert_eq!(decision.scheduled_day(), Weekday::Friday);
    assert_eq!(decision.observed_day(), Weekday::Monday);
    assert_eq!(
        decision.to_string(),
        "not due: scheduled Friday, observed Monday"
    );
}

#[test]
fn repeated_evaluation_is_pure_and_does_not_consume_a_week() {
    let scheduler = WeeklyScheduler::new(WeeklySchedule::new(Weekday::Wednesday));

    assert!(scheduler.should_run(Weekday::Wednesday));
    assert!(scheduler.should_run(Weekday::Wednesday));
    assert!(!scheduler.should_run(Weekday::Tuesday));
}
