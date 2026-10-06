use super::*;

#[test]
fn quick_entry_strips_keyword_after_extra_whitespace() {
    // Word offsets must track real separator widths: a double space or a
    // leading space used to shift remove_range off the keyword, so the date
    // word survived inside the saved title.
    let tomorrow = key_of(Local::now().date_naive() + Duration::days(1));
    let (clean, date) = parse_quick_entry_capture("купить  молоко   завтра", None);
    assert_eq!(date.as_deref(), Some(tomorrow.as_str()));
    assert_eq!(clean, "купить молоко");

    let (clean, date) = parse_quick_entry_capture("  завтра убрать", None);
    assert_eq!(date.as_deref(), Some(tomorrow.as_str()));
    assert_eq!(clean, "убрать");
}

#[test]
fn quick_entry_strips_keyword_single_space() {
    let tomorrow = key_of(Local::now().date_naive() + Duration::days(1));
    let (clean, date) = parse_quick_entry_capture("купить молоко завтра", None);
    assert_eq!(date.as_deref(), Some(tomorrow.as_str()));
    assert_eq!(clean, "купить молоко");
}

#[test]
fn quick_entry_through_n_days() {
    let expected = key_of(Local::now().date_naive() + Duration::days(10));
    let (clean, date) = parse_quick_entry_capture("позвонить  через 10 дней", None);
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
        let (clean, date) = parse_quick_entry_capture(title, None);
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
        let (clean, date) = parse_quick_entry_capture(&format!("сдать отчёт {word}"), None);
        assert_eq!(date.as_deref(), Some(expected.as_str()), "{word}");
        assert_eq!(clean, "сдать отчёт", "{word}");
    }
}

#[test]
fn quick_entry_does_not_treat_sredi_as_wednesday() {
    // "среди" is a common preposition ("among"), not a declension of "среда":
    // stem "сред" + suffix "и" accidentally matched, so the word was stripped
    // from the title and the task silently scheduled for Wednesday.
    let (clean, date) = parse_quick_entry_capture("обсудить бюджет среди команды", None);
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
        let (clean, date) = parse_quick_entry_capture(title, None);
        assert_eq!(date, None, "{title} must not produce a date");
        assert_eq!(clean, title);
    }
}

#[test]
fn next_recurrence_survives_absurd_rules() {
    // Engine-fed rules can carry arbitrary intervals; absurd ones must not
    // panic (or hang on a 4-billion-step month loop) when a task completes.
    let mut t = new_todo("t", "x");
    t.status = Status::Todo;
    t.scheduled_date = Some(today_key());
    for (frequency, interval, days_of_week) in [
        (0, u32::MAX, vec![]),
        (1, u32::MAX, vec![]),
        (2, u32::MAX, vec![]),
        (3, u32::MAX, vec![]),
        // Weekday ids outside 1..=7 can never match — no next occurrence.
        (1, 1, vec![0]),
    ] {
        let rule = RecurrenceRule {
            frequency,
            interval,
            recurrence_type: 0,
            days_of_week,
        };
        assert_eq!(next_recurrence_date(&rule, &t), None);
    }
}

#[test]
fn after_completion_recurrence_counts_from_completion_day() {
    // recurrenceType=1 ("после выполнения") repeats relative to the day the
    // task was completed, not its old scheduled date: a task scheduled 10
    // days ago but completed today repeats tomorrow, not 9 days ago.
    let mut t = new_todo("t", "x");
    t.status = Status::Todo;
    t.scheduled_date = Some(day_key(-10));
    t.completed_at = Some(chrono::Utc::now().to_rfc3339());
    let rule = RecurrenceRule {
        frequency: 0,
        interval: 1,
        recurrence_type: 1,
        days_of_week: vec![],
    };
    assert_eq!(
        next_recurrence_date(&rule, &t).as_deref(),
        Some(day_key(1).as_str())
    );
}

#[test]
fn fixed_recurrence_counts_from_schedule() {
    // recurrenceType=0 ("по расписанию") chains from the scheduled date even
    // when completion happens later.
    let mut t = new_todo("t", "x");
    t.status = Status::Todo;
    t.scheduled_date = Some(day_key(-10));
    t.completed_at = Some(chrono::Utc::now().to_rfc3339());
    let rule = RecurrenceRule {
        frequency: 0,
        interval: 1,
        recurrence_type: 0,
        days_of_week: vec![],
    };
    assert_eq!(
        next_recurrence_date(&rule, &t).as_deref(),
        Some(day_key(-9).as_str())
    );
}

#[test]
fn completed_day_counts_only_genuine_completions() {
    // The statistics heatmap/totals read `completed_day`: canceled tasks
    // carry a completed_at stamp ("closed at") but are not completions, and
    // trashed tasks must not count anywhere.
    let stamp = iso_at(0, 12, 0);
    let key = today_key();

    let mut done = new_todo("a", "x");
    done.status = Status::Done;
    done.is_completed = true;
    done.completed_at = Some(stamp.clone());
    assert_eq!(completed_day(&done), parse_key(&key));

    let mut canceled = done.clone();
    canceled.id = "b".into();
    canceled.status = Status::Canceled;
    canceled.is_completed = false;
    canceled.is_cancelled = true;
    assert_eq!(completed_day(&canceled), None);

    let mut trashed = done.clone();
    trashed.id = "c".into();
    trashed.is_trashed = true;
    assert_eq!(completed_day(&trashed), None);

    let mut no_stamp = done.clone();
    no_stamp.id = "d".into();
    no_stamp.completed_at = None;
    assert_eq!(completed_day(&no_stamp), None);
}

fn project(id: &str, title: &str) -> Project {
    Project {
        id: id.into(),
        title: title.into(),
        status: 0,
        deadline: None,
        sort_order: 0,
        area_id: None,
    }
}

#[test]
fn project_mention_basic() {
    let (clean, pid) = parse_project_mention("Купить молоко @Дом", &[project("p1", "Дом")]);
    assert_eq!(pid.as_deref(), Some("p1"));
    assert_eq!(clean, "Купить молоко");
}

#[test]
fn project_mention_handles_lowercase_expansion() {
    // 'İ' lowercases to "i̇" (2 chars / 3 UTF-8 bytes): byte offsets taken
    // from `title.to_lowercase()` do not map back onto `title`, so the cut
    // range lands mid-mention and leaves a dangling '@' (or skips the strip).
    let (clean, pid) = parse_project_mention("İX @ab!", &[project("p1", "ab")]);
    assert_eq!(pid.as_deref(), Some("p1"));
    assert_eq!(clean, "İX !");

    let (clean, pid) = parse_project_mention("İX @ab", &[project("p1", "ab")]);
    assert_eq!(pid.as_deref(), Some("p1"));
    assert_eq!(clean, "İX");
}

#[test]
fn logbook_sorts_completed_instants_not_strings() {
    // RFC 3339 instants with different offsets do not sort like the wall clock
    // as raw strings: "+03:00" compares above "Z" suffixes even though it marks
    // an *earlier* instant. The Logbook must order by the parsed instant.
    let mut earlier = new_todo("a", "done earlier (larger local clock)");
    earlier.is_completed = true;
    earlier.status = Status::Done;
    earlier.completed_at = Some("2020-01-10T23:30:00+03:00".into()); // 20:30Z
    let mut later = new_todo("b", "done later (smaller local clock)");
    later.is_completed = true;
    later.status = Status::Done;
    later.completed_at = Some("2020-01-10T21:00:00Z".into());

    let got: Vec<usize> = filter_idx(SmartList::Logbook, &[earlier.clone(), later.clone()]);
    assert_eq!(got, [1, 0], "filter_idx must order Logbook by instant");
}

/// A task dated earlier this week is overdue AND inside the week bounds; the
/// Week list used to show it twice (Tuesday through Sunday), and the two rows
/// shared one element id, so hovering them flapped forever.
#[test]
fn week_list_has_no_duplicate_tasks() {
    let todos: Vec<Todo> = (-6..=6)
        .map(|offset| {
            let mut t = todo(format!("t{offset}"), "task");
            t.status = Status::Todo;
            t.scheduled_date = Some(day_key(offset));
            t
        })
        .collect();
    let mut seen = filter_idx(SmartList::Week, &todos);
    assert!(!seen.is_empty());
    let listed = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), listed, "Week list repeats a task");
}
