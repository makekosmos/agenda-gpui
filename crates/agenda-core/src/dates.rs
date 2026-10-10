// Date helpers (mirror taskLifecycle localDateKey / taskDate).
use crate::types::Todo;
use chrono::{Datelike, Duration, FixedOffset, Local, NaiveDate};

/// The clock input every date-sensitive function takes: the local civil day
/// plus the local UTC offset (needed to interpret RFC 3339 instants as local
/// days). Callers construct it from the real clock via [`LocalDay::now`] or
/// pin it in tests via [`LocalDay::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalDay {
    pub today: NaiveDate,
    pub offset: FixedOffset,
}

impl LocalDay {
    pub fn now() -> Self {
        let now = Local::now();
        Self {
            today: now.date_naive(),
            offset: *now.offset(),
        }
    }

    pub fn new(today: NaiveDate, offset: FixedOffset) -> Self {
        Self { today, offset }
    }

    pub fn today_key(&self) -> String {
        key_of(self.today)
    }

    /// Day key `offset` days from `today` — dev seeds and tests use this.
    pub fn day_key(&self, offset: i64) -> String {
        key_of(self.today + Duration::days(offset))
    }

    /// RFC 3339-shaped local stamp `offset` days from `today` at `h:m`.
    pub fn iso_at(&self, offset: i64, h: u32, m: u32) -> String {
        let d = self.today + Duration::days(offset);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:00",
            d.year(),
            d.month(),
            d.day(),
            h,
            m
        )
    }

    pub fn week_bounds(&self) -> Option<(String, String)> {
        week_bounds(&self.today_key())
    }

    pub fn next_week_bounds(&self) -> Option<(String, String)> {
        next_week_bounds(&self.today_key())
    }

    pub fn next_monday_key(&self) -> Option<String> {
        next_monday_key(&self.today_key())
    }
}

pub fn key_of(d: NaiveDate) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

pub fn parse_key(key: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(key, "%Y-%m-%d").ok()
}

/// The day an RFC 3339 stamp expresses at `offset` — pure, so tests pin a
/// zone instead of depending on the machine's `Local`. The local-time rule is
/// deliberate: old writers emitted `toISOString()` of local midnights, and
/// ark-core's `canonical_types::normalize` applies the same rule on read.
pub fn stamp_day_in(v: &str, offset: FixedOffset) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(v)
        .ok()
        .map(|dt| key_of(dt.with_timezone(&offset).date_naive()))
}

/// dateOnly(): accepts "YYYY-MM-DD" or ISO datetime → "YYYY-MM-DD".
/// Stamps carrying an explicit offset are instants: interpret them in
/// local time (dayjs parity) instead of truncating the UTC representation,
/// which shifts the day for anyone outside UTC around midnight.
pub fn date_only(value: &Option<String>, day: &LocalDay) -> Option<String> {
    let v = value.as_ref()?;
    if let Some(day) = stamp_day_in(v, day.offset) {
        return Some(day);
    }
    // A ≥10-char prefix is not proof of a date: malformed stamps from other
    // clients became lexicographic "date" keys, hiding tasks from Inbox and
    // sorting them into phantom groups. Keep only parseable YYYY-MM-DD.
    v.get(..10)
        .filter(|s| parse_key(s).is_some())
        .map(str::to_owned)
}

/// taskDate(): deadline wins over scheduled; different dates = conflict.
pub fn task_date(t: &Todo, day: &LocalDay) -> (Option<String>, bool) {
    let scheduled = date_only(&t.scheduled_date, day);
    let deadline = date_only(&t.deadline, day);
    if scheduled.is_some() && deadline.is_some() && scheduled != deadline {
        return (None, true);
    }
    (deadline.or(scheduled), false)
}

/// Next ISO week bounds (Monday..=Sunday) as day keys.
pub fn next_week_bounds(today: &str) -> Option<(String, String)> {
    let (mon, sun) = week_bounds(today)?;
    let mon = parse_key(&mon)? + Duration::days(7);
    let sun = parse_key(&sun)? + Duration::days(7);
    Some((key_of(mon), key_of(sun)))
}

/// Monday of next week as a day key — «следующая неделя» as a real date.
pub fn next_monday_key(today: &str) -> Option<String> {
    let (mon, _) = week_bounds(today)?;
    let mon = parse_key(&mon)? + Duration::days(7);
    Some(key_of(mon))
}

/// Current ISO week bounds (Monday..=Sunday) as day keys.
pub fn week_bounds(today: &str) -> Option<(String, String)> {
    let d = parse_key(today)?;
    let mon = d - Duration::days(d.weekday().num_days_from_monday() as i64);
    Some((key_of(mon), key_of(mon + Duration::days(6))))
}

// ---------------------------------------------------------------------------
// Russian date formatting (Intl.DateTimeFormat ru-RU equivalents)
// ---------------------------------------------------------------------------

pub const MONTH_SHORT: [&str; 12] = [
    "янв.",
    "февр.",
    "мар.",
    "апр.",
    "мая",
    "июня",
    "июля",
    "авг.",
    "сент.",
    "окт.",
    "нояб.",
    "дек.",
];
pub const MONTH_LONG: [&str; 12] = [
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];

/// {day: numeric, month: short} then strip trailing dot → "18 сент"
pub fn fmt_day_month(key: &str) -> String {
    match parse_key(key) {
        Some(d) => {
            let m = MONTH_SHORT[(d.month() - 1) as usize];
            let s = format!("{} {}", d.day(), m);
            s.trim_end_matches('.').to_string()
        }
        None => key.to_string(),
    }
}
