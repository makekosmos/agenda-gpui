//! Regressions for KOS-232 residual bugs.
//!
//! Covered:
//! - `date_only` truncated offset-carrying timestamps instead of
//!   interpreting them in local time — completion stamps written as
//!   `Utc::now().to_rfc3339()` (and Engine `completedAt`) land on the wrong
//!   day around local midnight, skewing Today/Logbook/statistics and the
//!   after-completion recurrence base;
//! - «через N …» capture treated every unit except неделя as days —
//!   "через 2 часа" scheduled +2 days, "через 3 месяца" +3 days; bare units
//!   ("через неделю") were never captured at all;
//! - statistics share overlay: the dim backdrop dismissed the card but did
//!   not occlude, so the same press also reached the stats page beneath —
//!   clicking the dim where «Поделиться» sits closed and immediately
//!   reopened the overlay (same class as the QE backdrop fix from KOS-216,
//!   a missed instance);
//! - quick-entry date dropdown: `DropKind::QeDate` resolved its checked row
//!   from the owning todo, which is always `None` for quick entry, so
//!   «Без даты» stayed checked no matter what date chip was picked.

use gpui::TestAppContext;

use crate::app::{DropKind, DropState, Route};
use crate::ui_tests::{click, launch, redraw};
use agenda_core::{date_only, key_of, parse_quick_entry_capture, LocalDay};
use chrono::{Duration, Local};

#[test]
fn date_only_interprets_offset_stamps_in_local_time() {
    // "2026-06-01T01:00+14:00" is 2026-05-31T11:00Z. Truncating the string
    // reports June 1, but the instant's local date is May 31 in every
    // timezone west of UTC+14 — the same skew a `Utc::now().to_rfc3339()`
    // completion stamp produces around local midnight.
    let stamp = "2026-06-01T01:00:00+14:00".to_string();
    let local = chrono::DateTime::parse_from_rfc3339(&stamp)
        .unwrap()
        .with_timezone(&Local)
        .date_naive();
    assert_eq!(
        date_only(&Some(stamp), &LocalDay::now()),
        Some(key_of(local))
    );

    // Naive and date-only values keep their literal day.
    assert_eq!(
        date_only(&Some("2026-06-01T01:00:00".into()), &LocalDay::now()).as_deref(),
        Some("2026-06-01")
    );
    assert_eq!(
        date_only(&Some("2026-06-01".into()), &LocalDay::now()).as_deref(),
        Some("2026-06-01")
    );
}

/// The day a stamp expresses is the day in the reader's zone — shared vectors
/// with ark-core `canonical_types::normalize::tests`. Fixed offsets keep this
/// deterministic on any machine.
#[test]
fn stamp_day_matches_the_engine_rule() {
    use agenda_core::stamp_day_in;
    let at = |secs: i32| chrono::FixedOffset::east_opt(secs).unwrap();
    let cases: &[(&str, i32, &str)] = &[
        ("2026-05-15T21:00:00Z", 3 * 3600, "2026-05-16"),
        ("2026-05-15T21:00:00Z", -5 * 3600, "2026-05-15"),
        ("2026-05-15T23:30:00+03:00", 3 * 3600, "2026-05-15"),
        ("2026-05-15T23:30:00+03:00", -5 * 3600, "2026-05-15"),
        ("2026-05-16T00:30:00-05:00", -5 * 3600, "2026-05-16"),
        ("2026-05-16T00:30:00-05:00", 14 * 3600, "2026-05-16"),
    ];
    for (stamp, secs, expected) in cases {
        assert_eq!(
            stamp_day_in(stamp, at(*secs)).as_deref(),
            Some(*expected),
            "{stamp} at offset {secs}"
        );
    }
    for bad in ["soon", "2026-13-40", "2026-05-15T25:00:00Z"] {
        assert!(stamp_day_in(bad, at(0)).is_none(), "{bad}");
    }
}

#[test]
fn quick_entry_through_time_units_stay_today() {
    // The model stores dates only, so "через N часов/минут" must land on
    // today — it used to fall into the days branch and schedule N days out.
    let today = LocalDay::now().today;
    for title in [
        "позвонить через 2 часа",
        "написать через 30 минут",
        "вернуться через час",
    ] {
        let (_, date) = parse_quick_entry_capture(title, None, &LocalDay::now());
        assert_eq!(date.as_deref(), Some(key_of(today).as_str()), "{title}");
    }
    let (clean, _) =
        parse_quick_entry_capture("позвонить через 2 часа маме", None, &LocalDay::now());
    assert_eq!(clean, "позвонить маме");
}

#[test]
fn quick_entry_through_months_and_years() {
    // "через N месяцев/лет" used to schedule N *days* out.
    let today = Local::now().date_naive();
    let in3 = key_of(today.checked_add_months(chrono::Months::new(3)).unwrap());
    let (clean, date) =
        parse_quick_entry_capture("оплатить через 3 месяца", None, &LocalDay::now());
    assert_eq!(date.as_deref(), Some(in3.as_str()));
    assert_eq!(clean, "оплатить");

    let in24 = key_of(today.checked_add_months(chrono::Months::new(24)).unwrap());
    let (_, date) = parse_quick_entry_capture("пересмотреть через 2 года", None, &LocalDay::now());
    assert_eq!(date.as_deref(), Some(in24.as_str()));

    let in60 = key_of(today.checked_add_months(chrono::Months::new(60)).unwrap());
    let (_, date) = parse_quick_entry_capture("юбилей через 5 лет", None, &LocalDay::now());
    assert_eq!(date.as_deref(), Some(in60.as_str()));
}

#[test]
fn quick_entry_through_bare_units() {
    // «через неделю/месяц/год/день» (unit without a numeral) reads as n=1.
    let today = Local::now().date_naive();
    for (title, want) in [
        ("созвон через неделю", today + Duration::days(7)),
        ("созвон через день", today + Duration::days(1)),
        (
            "отчёт через месяц",
            today.checked_add_months(chrono::Months::new(1)).unwrap(),
        ),
        (
            "отчёт через год",
            today.checked_add_months(chrono::Months::new(12)).unwrap(),
        ),
    ] {
        let (clean, date) = parse_quick_entry_capture(title, None, &LocalDay::now());
        assert_eq!(date.as_deref(), Some(key_of(want).as_str()), "{title}");
        assert!(!clean.contains("через"), "{title} → {clean}");
    }

    // A non-unit word after «через» is still not a capture.
    let (clean, date) =
        parse_quick_entry_capture("пережить через неприятности", None, &LocalDay::now());
    assert_eq!(date, None);
    assert_eq!(clean, "пережить через неприятности");
}

#[gpui::test]
fn share_overlay_dismiss_does_not_leak_to_page(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| a.navigate(Route::Statistics));
    redraw(cx);

    click(cx, "stat-share");
    assert!(agenda.read_with(cx, |a, _| a.share_open));

    // Same spot again: only the dim may see this press. Without occlusion
    // the «Поделиться» button underneath stays hovered and reopens the card.
    click(cx, "stat-share");
    assert!(
        !agenda.read_with(cx, |a, _| a.share_open),
        "a press on the dim must dismiss the share overlay, not re-open it"
    );
}

#[gpui::test]
fn qe_date_dropdown_checks_current_value(cx: &mut TestAppContext) {
    let (agenda, cx) = launch(cx);
    agenda.update(cx, |a, _| {
        a.qe_week = true;
        a.qe_date_touched = true;
        a.dropdown = Some(DropState {
            kind: DropKind::QeDate,
            x: 100.,
            y: 100.,
            todo_id: None,
        });
    });
    redraw(cx);

    let json = cx
        .update(|window, _| window.debug_a11y_tree_json())
        .expect("a11y tree");
    let tree: serde_json::Value = serde_json::from_str(&json).expect("valid a11y json");
    let selected = |label: &str| -> Option<bool> {
        tree["nodes"]
            .as_object()
            .expect("nodes object")
            .values()
            .find(|n| {
                n["aria"]["role"].as_str() == Some("MenuItem")
                    && n["aria"]["label"].as_str() == Some(label)
            })
            .and_then(|n| n["aria"]["selected"].as_bool())
    };
    assert_eq!(selected("Эта неделя"), Some(true));
    assert_eq!(selected("Следующая неделя"), Some(false));
    assert_eq!(selected("Потом"), Some(false));
    assert_eq!(selected("Без даты"), Some(false));
}
