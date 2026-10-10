// Recurrence (createNextRecurrence subset).
use crate::dates::{date_only, key_of, parse_key, task_date, LocalDay};
use crate::types::{RecurrenceRule, Todo};
use chrono::{Datelike, Duration};

/// Next occurrence for a completed recurring task. Rules come from Engine
/// data, so interval/weekday values are untrusted: checked arithmetic keeps
/// absurd rules from panicking or looping for billions of months — they
/// simply produce no next occurrence.
pub fn next_recurrence_date(rule: &RecurrenceRule, t: &Todo, day: &LocalDay) -> Option<String> {
    // recurrenceType=1 ("после выполнения") counts the interval from the
    // completion day; type=0 ("по расписанию") chains the fixed schedule
    // regardless of when the task was actually completed.
    let base = if rule.recurrence_type == 1 {
        date_only(&t.completed_at, day)
            .and_then(|d| parse_key(&d))
            .unwrap_or(day.today)
    } else {
        task_date(t, day)
            .0
            .and_then(|d| parse_key(&d))
            .unwrap_or(day.today)
    };
    let interval = i64::from(rule.interval);
    let next = match rule.frequency {
        0 => Duration::try_days(interval).and_then(|d| base.checked_add_signed(d)),
        1 => {
            if rule.days_of_week.is_empty() {
                interval
                    .checked_mul(7)
                    .and_then(Duration::try_days)
                    .and_then(|d| base.checked_add_signed(d))
            } else {
                // Next selected weekday strictly after base.
                let mut d = base;
                let mut found = None;
                for _ in 0..8 {
                    d = d.checked_add_signed(Duration::days(1))?;
                    let wd = d.weekday().num_days_from_monday() as u8 + 1;
                    if rule.days_of_week.contains(&wd) {
                        found = Some(d);
                        break;
                    }
                }
                found
            }
        }
        2 => base.checked_add_months(chrono::Months::new(rule.interval)),
        _ => base.checked_add_months(chrono::Months::new(rule.interval.checked_mul(12)?)),
    };
    next.map(key_of)
}
