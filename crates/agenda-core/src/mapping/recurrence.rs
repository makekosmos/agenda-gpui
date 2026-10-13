use crate::types::Todo;
use serde_json::{json, Value};

const FREQUENCIES: [&str; 4] = ["daily", "weekly", "monthly", "yearly"];

/// The Engine's `recurrence` object as Agenda's rule shape (`null` when absent).
pub(super) fn read_recurrence(value: &Value) -> Value {
    let Some(rule) = value.as_object() else {
        return Value::Null;
    };
    let frequency = FREQUENCIES
        .iter()
        .position(|f| Some(*f) == rule.get("frequency").and_then(Value::as_str))
        .unwrap_or(0);
    json!({
        "frequency": frequency,
        "interval": rule.get("interval").filter(|v| v.is_u64()).cloned().unwrap_or(json!(1)),
        "recurrence_type": u8::from(rule.get("kind") == Some(&json!("afterCompletion"))),
        "days_of_week": rule.get("daysOfWeek").filter(|v| v.is_array()).cloned().unwrap_or(json!([])),
    })
}

/// Write Agenda's rule into the Engine's `recurrence`. The Engine owns the
/// time zone and end condition, so both carry over from the stored rule.
pub(super) fn write_recurrence(props: &mut Value, todo: &Todo) {
    let current = props["recurrence"].clone();
    props["recurrence"] = match todo.recurrence.as_ref() {
        None => Value::Null,
        Some(rule) => {
            // Agenda weekdays are ISO 1..7 (Monday = 1), the same as core.task.
            let mut days: Vec<u8> = rule
                .days_of_week
                .iter()
                .copied()
                .filter(|d| (1..=7).contains(d))
                .collect();
            days.sort_unstable();
            days.dedup();
            json!({
                "kind": if rule.recurrence_type == 1 { "afterCompletion" } else { "calendar" },
                "frequency": FREQUENCIES[rule.frequency.min(3) as usize],
                "interval": rule.interval.max(1),
                "daysOfWeek": days,
                "timeZone": current["timeZone"].clone(),
                "end": current["end"].clone(),
            })
        }
    };
}
