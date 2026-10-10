//! `n` todos with random-character titles and randomized fields for
//! `AGENDA_DEVTASKS` (xorshift64*, no external deps).
use super::*;

/// Dev tool: `n` todos with random-character titles and randomized fields
/// (xorshift64*, no external deps). ~55% are completed, spread over the past
/// year so the statistics heatmap has data to show. Ids are namespaced as
/// `dev-gen-{batch}-{i}` so batches never collide.
pub fn gen_random_todos(n: usize, seed: u64, batch: u64, first_sort: i32) -> Vec<Todo> {
    let day = LocalDay::now();
    const PROJECTS: [&str; 3] = ["dev-proj-release", "dev-proj-home", "dev-proj-someday"];
    const TAGS: [&str; 3] = ["dev-tag-urgent", "dev-tag-focus", "dev-tag-home"];

    let mut s = (seed | 1).wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut rng = move || {
        s ^= s >> 12;
        s ^= s << 25;
        s ^= s >> 27;
        s = s.wrapping_mul(0x2545_F491_4F6C_DD1D);
        s
    };
    fn rand_chars(rng: &mut impl FnMut() -> u64, words_max: u64) -> String {
        let words = 1 + rng() % words_max;
        (0..words)
            .map(|_| {
                let len = 2 + rng() % 9;
                (0..len)
                    .map(|_| (b'a' + (rng() % 26) as u8) as char)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    (0..n)
        .map(|i| {
            let mut t = todo(format!("dev-gen-{batch}-{i}"), &rand_chars(&mut rng, 4));
            t.created_at = day.iso_at(
                -((rng() % 365) as i64),
                (rng() % 24) as u32,
                (rng() % 60) as u32,
            );
            t.sort_order = first_sort + i as i32;
            if rng() % 100 < 55 {
                t.status = Status::Done;
                t.is_completed = true;
                t.completed_at = Some(day.iso_at(
                    -((rng() % 365) as i64),
                    (rng() % 24) as u32,
                    (rng() % 60) as u32,
                ));
            } else {
                t.status = match rng() % 100 {
                    0..=19 => Status::Inbox,
                    20..=59 => Status::Todo,
                    60..=74 => Status::Started,
                    75..=89 => Status::Deferred,
                    _ => Status::Canceled,
                };
                match t.status {
                    Status::Canceled => {
                        t.is_cancelled = true;
                        t.completed_at = Some(day.iso_at(-((rng() % 180) as i64), 12, 0));
                    }
                    Status::Deferred => t.is_someday = rng() % 100 < 60,
                    _ => {}
                }
                if rng() % 100 < 6 {
                    t.is_today = true;
                }
                if rng() % 100 < 40 {
                    t.scheduled_date = Some(day.day_key((rng() % 76) as i64 - 30));
                }
                if rng() % 100 < 15 {
                    t.deadline = Some(day.day_key((rng() % 61) as i64 - 10));
                }
                if rng() % 100 < 3 {
                    t.is_trashed = true;
                }
            }
            if rng() % 100 < 30 {
                t.priority = 1 + (rng() % 3) as u8;
            }
            if rng() % 100 < 35 {
                t.project_id = Some(PROJECTS[(rng() % 3) as usize].into());
            }
            if rng() % 100 < 20 {
                t.tag_ids = vec![TAGS[(rng() % 3) as usize].into()];
            }
            if rng() % 100 < 15 {
                t.notes = Some(rand_chars(&mut rng, 12));
            }
            if rng() % 100 < 30 {
                t.fuel_cost = Some(5.0 + (rng() % 60) as f64);
                t.significance = Some(1 + (rng() % 10) as u8);
            }
            t
        })
        .collect()
}
