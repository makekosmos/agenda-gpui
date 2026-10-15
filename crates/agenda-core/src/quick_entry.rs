// Quick-entry parsing (parseProjectMention + simplified parseQuickEntryCapture).
use crate::dates::{key_of, LocalDay};
use crate::types::Project;
use chrono::{Datelike, Duration, NaiveDate};

/// taskLifecycle.parseProjectMention: find "@<project title>" word-bounded mention.
pub fn parse_project_mention(title: &str, projects: &[Project]) -> (String, Option<String>) {
    let mut sorted: Vec<&Project> = projects.iter().collect();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.title.len()));
    for p in sorted {
        let marker = format!("@{}", p.title.to_lowercase());
        // Search in `title`'s own coordinates: a lowercased copy can differ in
        // byte length ('İ' → "i̇"), which would shift the cut range and corrupt
        // the remaining text.
        for (start, _) in title.char_indices() {
            if !title[..start]
                .chars()
                .last()
                .is_none_or(|c| c.is_whitespace())
            {
                continue;
            }
            let mut acc = String::new();
            let mut end = start;
            for (off, ch) in title[start..].char_indices() {
                acc.extend(ch.to_lowercase());
                end = start + off + ch.len_utf8();
                if acc == marker || !marker.starts_with(acc.as_str()) {
                    break;
                }
            }
            if acc != marker {
                continue;
            }
            if title[end..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                continue;
            }
            let clean = format!("{}{}", &title[..start], &title[end..]);
            let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
            return (
                if clean.is_empty() {
                    title.to_string()
                } else {
                    clean
                },
                Some(p.id.to_string()),
            );
        }
    }
    (title.to_string(), None)
}

/// Simplified parseQuickEntryCapture: handles Russian date keywords
/// (сегодня/завтра/послезавтра, weekday names → next occurrence, "через N дн/нед").
/// Full chrono-node parity is out of scope (documented gap).
pub fn parse_quick_entry_capture(
    title: &str,
    selected: Option<String>,
    day: &LocalDay,
) -> (String, Option<String>) {
    if selected.is_some() {
        return (title.to_string(), selected);
    }
    let today = day.today;
    // Word offsets must index `title` itself: separator runs can be wider than
    // one byte, and lowercasing may change byte length, so positions derived
    // from `title.to_lowercase()` do not map back onto `title`.
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut pos = 0usize;
    for w in title.split_whitespace() {
        let start = pos + title[pos..].find(w).unwrap_or(0);
        pos = start + w.len();
        words.push((start, w));
    }
    let mut date: Option<NaiveDate> = None;
    let mut remove_range: Option<(usize, usize)> = None;
    let ru_weekdays = [
        ("понедельник", 0u32),
        ("вторник", 1),
        ("сред", 2),
        ("четверг", 3),
        ("пятниц", 4),
        ("суббот", 5),
        ("воскресень", 6),
    ];
    // Russian weekday declensions: nominative + case endings accepted after
    // each stem above (среда/среду/средой, пятницу/пятницей, воскресенья...).
    const WEEKDAY_SUFFIXES: &[&str] = &[
        "", "а", "у", "е", "и", "ы", "ю", "я", "ой", "ей", "ом", "ем", "ам", "ям",
    ];
    for (i, (start, w)) in words.iter().enumerate() {
        let end = *start + w.len();
        let lw = w.to_lowercase();
        let trimmed = lw.trim_matches(|c: char| !c.is_alphanumeric());
        let d = match trimmed {
            "сегодня" => Some(today),
            "завтра" => Some(today + Duration::days(1)),
            "послезавтра" => Some(today + Duration::days(2)),
            _ if trimmed.starts_with("через") || trimmed == "через" => None,
            _ => ru_weekdays
                .iter()
                .find(|(name, _)| {
                    // Bare prefix matching treats "средний"/"субботник" as
                    // weekday names; only declension endings may follow a stem.
                    // "среди" is the exception: a common preposition, not a
                    // declension of "среда" — the "и" ending is valid only for
                    // the other stems.
                    trimmed.strip_prefix(name).is_some_and(|s| {
                        WEEKDAY_SUFFIXES.contains(&s) && !(*name == "сред" && s == "и")
                    })
                })
                .map(|(_, wd)| {
                    let cur = today.weekday().num_days_from_monday();
                    let delta = (7 + wd - cur) % 7;
                    today + Duration::days(if delta == 0 { 7 } else { delta } as i64)
                }),
        };
        if let Some(d) = d {
            date = Some(d);
            remove_range = Some((*start, end));
            break;
        }
        if trimmed == "через" {
            if let Some((s2, w2)) = words.get(i + 1) {
                let numeral = w2
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .parse::<i64>()
                    .ok();
                // A bare unit word reads as n=1 ("через неделю", "через
                // час"); anything else after «через» is not a capture.
                let bare = w2.to_lowercase();
                let parsed = match numeral {
                    Some(n) => Some((n, i + 2)),
                    None if through_stem(&bare).is_some() => Some((1, i + 1)),
                    None => None,
                };
                if let Some((n, unit_i)) = parsed {
                    let unit = words
                        .get(unit_i)
                        .map(|(_, u)| u.to_lowercase())
                        .unwrap_or_default();
                    let months = |m: i64| {
                        u32::try_from(m)
                            .ok()
                            .and_then(|m| today.checked_add_months(chrono::Months::new(m)))
                    };
                    // The model stores dates only: sub-day units ("через 2
                    // часа") can only mean today. Month/year units used to
                    // fall into the days branch and scheduled N days out.
                    let target = match through_stem(&unit) {
                        Some(0) => n
                            .checked_mul(7)
                            .and_then(Duration::try_days)
                            .and_then(|d| today.checked_add_signed(d)),
                        Some(1) => months(n),
                        Some(2) => n.checked_mul(12).and_then(months),
                        Some(3) => Some(today),
                        // день/дня/дней and anything unrecognized after a
                        // numeral keep the original days fallback.
                        _ => Duration::try_days(n).and_then(|d| today.checked_add_signed(d)),
                    };
                    // `NaiveDate + duration` panics on out-of-range values —
                    // an unrepresentable offset is simply "no date captured".
                    if let Some(d) = target {
                        date = Some(d);
                        let e = words
                            .get(unit_i)
                            .map(|(s, u)| s + u.len())
                            .unwrap_or(s2 + w2.len());
                        remove_range = Some((*start, e));
                        break;
                    }
                }
            }
        }
    }
    let Some(d) = date else {
        return (title.to_string(), None);
    };
    let clean = if let Some((s, e)) = remove_range {
        let mut t = String::new();
        if s <= title.len()
            && e <= title.len()
            && title.is_char_boundary(s)
            && title.is_char_boundary(e)
        {
            t.push_str(&title[..s]);
            t.push_str(&title[e..]);
        } else {
            t = title.to_string();
        }
        t.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        title.to_string()
    };
    (
        if clean.is_empty() {
            title.to_string()
        } else {
            clean
        },
        Some(key_of(d)),
    )
}

/// Unit stems after «через»: 0=неделя 1=месяц 2=год/лет 3=час/мин/сек 4=день.
fn through_stem(unit: &str) -> Option<u8> {
    let u = unit.trim_matches(|c: char| !c.is_alphanumeric());
    Some(match u {
        _ if u.starts_with("недел") => 0,
        _ if u.starts_with("месяц") => 1,
        _ if u.starts_with("год") || u.starts_with("лет") => 2,
        _ if u.starts_with("час") || u.starts_with("мин") || u.starts_with("секунд") => {
            3
        }
        _ if u.starts_with("дн") || u.starts_with("день") => 4,
        _ => return None,
    })
}
