//! Deterministic application boundary for selecting the weekly report day.
//!
//! The scheduler receives a weekday from an outer runtime and decides whether
//! the Weekly Radar use case should be triggered. It does not read the clock,
//! calculate research facts, persist snapshots, render messages, publish, or
//! retry a report.

use std::fmt;

#[cfg(test)]
#[path = "weekly_scheduler_test.rs"]
mod weekly_scheduler_test;

/// The seven calendar weekdays understood by the Weekly Radar schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

impl Weekday {
    /// All weekdays in their conventional Monday-to-Sunday order.
    pub const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];

    /// Returns the stable human-readable weekday label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
            Self::Sunday => "Sunday",
        }
    }
}

impl fmt::Display for Weekday {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Configuration for one weekly trigger day.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeeklySchedule {
    day_of_week: Weekday,
}

impl WeeklySchedule {
    /// Creates a weekly schedule for the supplied weekday.
    pub const fn new(day_of_week: Weekday) -> Self {
        Self { day_of_week }
    }

    /// Returns the configured trigger weekday.
    pub const fn day_of_week(self) -> Weekday {
        self.day_of_week
    }
}

impl Default for WeeklySchedule {
    fn default() -> Self {
        // Monday is the production publication day: 09:00 JST / 00:00 UTC.
        Self::new(Weekday::Monday)
    }
}

/// Result of evaluating one observed weekday against a weekly schedule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleDecision {
    /// The outer runner should trigger the Weekly Radar use case.
    Due {
        /// The weekday configured for the schedule.
        scheduled_day: Weekday,
        /// The weekday supplied by the outer runtime.
        observed_day: Weekday,
    },
    /// The outer runner should not trigger the use case for this observation.
    NotDue {
        /// The weekday configured for the schedule.
        scheduled_day: Weekday,
        /// The weekday supplied by the outer runtime.
        observed_day: Weekday,
    },
}

impl ScheduleDecision {
    /// Returns whether this result selects the Weekly Radar use case.
    pub const fn is_due(self) -> bool {
        matches!(self, Self::Due { .. })
    }

    /// Returns the configured weekday retained in this decision.
    pub const fn scheduled_day(self) -> Weekday {
        match self {
            Self::Due { scheduled_day, .. } | Self::NotDue { scheduled_day, .. } => scheduled_day,
        }
    }

    /// Returns the observed weekday retained in this decision.
    pub const fn observed_day(self) -> Weekday {
        match self {
            Self::Due { observed_day, .. } | Self::NotDue { observed_day, .. } => observed_day,
        }
    }
}

impl fmt::Display for ScheduleDecision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = if self.is_due() { "due" } else { "not due" };
        write!(
            formatter,
            "{state}: scheduled {}, observed {}",
            self.scheduled_day(),
            self.observed_day()
        )
    }
}

/// Application-level weekly trigger decision boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeeklyScheduler {
    schedule: WeeklySchedule,
}

impl WeeklyScheduler {
    /// Creates a scheduler from an explicit weekly schedule.
    pub const fn new(schedule: WeeklySchedule) -> Self {
        Self { schedule }
    }

    /// Returns the schedule used by this scheduler.
    pub const fn schedule(self) -> WeeklySchedule {
        self.schedule
    }

    /// Evaluates one weekday without reading system time or mutating state.
    pub fn evaluate(self, observed_day: Weekday) -> ScheduleDecision {
        let scheduled_day = self.schedule.day_of_week();
        if scheduled_day == observed_day {
            ScheduleDecision::Due {
                scheduled_day,
                observed_day,
            }
        } else {
            ScheduleDecision::NotDue {
                scheduled_day,
                observed_day,
            }
        }
    }

    /// Returns whether the supplied weekday should trigger the use case.
    pub fn should_run(self, observed_day: Weekday) -> bool {
        self.evaluate(observed_day).is_due()
    }
}

impl Default for WeeklyScheduler {
    fn default() -> Self {
        Self::new(WeeklySchedule::default())
    }
}
