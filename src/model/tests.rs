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
