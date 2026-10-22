// Unit tests for the moved model logic. The clock is pinned via
// `LocalDay::new` wherever the expectation is date-shaped; tests that only
// assert relative behavior still take a real `LocalDay::now()`.
use agenda_core::*;
use chrono::{Datelike, Duration, FixedOffset, Local, NaiveDate};

fn day() -> LocalDay {
    LocalDay::now()
}

fn pinned(today: &str) -> LocalDay {
    LocalDay::new(
        NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap(),
        FixedOffset::east_opt(0).unwrap(),
    )
}

#[test]
fn quick_entry_strips_keyword_after_extra_whitespace() {
    // Word offsets must track real separator widths: a double space or a
    // leading space used to shift remove_range off the keyword, so the date
    // word survived inside the saved title.
    let tomorrow = key_of(Local::now().date_naive() + Duration::days(1));
    let (clean, date) = parse_quick_entry_capture("купить  молоко   завтра", None, &day());
    assert_eq!(date.as_deref(), Some(tomorrow.as_str()));
    assert_eq!(clean, "купить молоко");

    let (clean, date) = parse_quick_entry_capture("  завтра убрать", None, &day());
    assert_eq!(date.as_deref(), Some(tomorrow.as_str()));
    assert_eq!(clean, "убрать");
}

#[test]
fn quick_entry_strips_keyword_single_space() {
    let tomorrow = key_of(Local::now().date_naive() + Duration::days(1));
    let (clean, date) = parse_quick_entry_capture("купить молоко завтра", None, &day());
    assert_eq!(date.as_deref(), Some(tomorrow.as_str()));
    assert_eq!(clean, "купить молоко");
}

#[test]
fn quick_entry_pinned_today() {
    // «завтра» / «через неделю» against a fixed day — the Android port must
    // produce the same keys from the same input date.
    let day = pinned("2026-10-10"); // суббота
    let (_, date) = parse_quick_entry_capture("купить молоко завтра", None, &day);
    assert_eq!(date.as_deref(), Some("2026-10-11"));
    let (_, date) = parse_quick_entry_capture("позвонить сегодня", None, &day);
    assert_eq!(date.as_deref(), Some("2026-10-10"));
    let (_, date) = parse_quick_entry_capture("созвон через неделю", None, &day);
    assert_eq!(date.as_deref(), Some("2026-10-17"));
    let (_, date) = parse_quick_entry_capture("отчёт в понедельник", None, &day);
    assert_eq!(date.as_deref(), Some("2026-10-12"));
    let (_, date) = parse_quick_entry_capture("в субботу", None, &day);
    assert_eq!(date.as_deref(), Some("2026-10-17")); // same weekday → next week
}

#[test]
fn quick_entry_through_n_days() {
    let expected = key_of(Local::now().date_naive() + Duration::days(10));
    let (clean, date) = parse_quick_entry_capture("позвонить  через 10 дней", None, &day());
    assert_eq!(date.as_deref(), Some(expected.as_str()));
    assert_eq!(clean, "позвонить");
}

#[test]
fn quick_entry_ignores_words_that_share_a_weekday_prefix() {
    // "средний" only shares the stem "сред" with "среда": it is not a date
    // keyword, so neither a date may be captured nor the word stripped.
    for title in [
        "задача средней сложности",
        "субботник в офисе",
        "пятничный дайджест",
        "средство от блёсток",
    ] {
        let (clean, date) = parse_quick_entry_capture(title, None, &day());
        assert_eq!(date, None, "{title} must not produce a date");
        assert_eq!(clean, title);
    }
}

#[test]
fn quick_entry_accepts_weekday_inflections() {
    let today = Local::now().date_naive();
    for (word, wd) in [
        ("понедельник", 0u32),
        ("понедельника", 0),
        ("вторник", 1),
        ("вторнику", 1),
        ("среда", 2),
        ("среду", 2),
        ("средой", 2),
        ("четверг", 3),
        ("четверга", 3),
        ("пятница", 4),
        ("пятницу", 4),
        ("пятницей", 4),
        ("суббота", 5),
        ("субботу", 5),
        ("воскресенье", 6),
        ("воскресенья", 6),
    ] {
        let cur = today.weekday().num_days_from_monday();
        let delta = (7 + wd - cur) % 7;
        let expected = key_of(today + Duration::days(if delta == 0 { 7 } else { delta } as i64));
        let (clean, date) = parse_quick_entry_capture(&format!("сдать отчёт {word}"), None, &day());
        assert_eq!(date.as_deref(), Some(expected.as_str()), "{word}");
        assert_eq!(clean, "сдать отчёт", "{word}");
    }
}

#[test]
fn quick_entry_does_not_treat_sredi_as_wednesday() {
    // "среди" is a common preposition ("among"), not a declension of "среда":
    // stem "сред" + suffix "и" accidentally matched, so the word was stripped
    // from the title and the task silently scheduled for Wednesday.
    let (clean, date) = parse_quick_entry_capture("обсудить бюджет среди команды", None, &day());
    assert_eq!(date, None);
    assert_eq!(clean, "обсудить бюджет среди команды");
}

#[test]
fn quick_entry_ignores_unrepresentable_offsets() {
    // Absurd offsets must not panic inside `Duration::days` or the
    // `NaiveDate + duration` add — an unrepresentable date is not a date.
    for title in [
        "позвонить через 99999999999 недель",
        "позвонить через 4000000000 дней",
        "позвонить через 2000000000000000000 недель",
    ] {
        let (clean, date) = parse_quick_entry_capture(title, None, &day());
        assert_eq!(date, None, "{title} must not produce a date");
        assert_eq!(clean, title);
    }
}

#[test]
fn completed_day_counts_only_genuine_completions() {
    // The statistics heatmap/totals read `completed_day`: open and trashed
    // tasks must not count anywhere.
    let day = day();
    let stamp = day.iso_at(0, 12, 0);
    let key = day.today_key();

    let mut done = new_todo("a", "x");
    done.status = Status::Done;
    done.is_completed = true;
    done.completed_at = Some(stamp.clone());
    assert_eq!(completed_day(&done, &day), parse_key(&key));

    let mut open = done.clone();
    open.id = "b".into();
    open.status = Status::Todo;
    open.is_completed = false;
    assert_eq!(completed_day(&open, &day), None);

    let mut trashed = done.clone();
    trashed.id = "c".into();
    trashed.is_trashed = true;
    assert_eq!(completed_day(&trashed, &day), None);

    let mut no_stamp = done.clone();
    no_stamp.id = "d".into();
    no_stamp.completed_at = None;
    assert_eq!(completed_day(&no_stamp, &day), None);
}
