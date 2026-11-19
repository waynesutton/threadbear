//! Tasks' due dates and priorities: the quick picks, the month calendar and
//! the chip's words. Due dates are plain days (`YYYY-MM-DD`), no time or zone.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};

/// The priorities, most pressing first, with their menu names.
pub const PRIORITIES: [(&str, &str); 4] = [("urgent", "Urgent"), ("high", "High"), ("normal", "Normal"), ("low", "Low")];

pub fn priority_name(priority: Option<&str>) -> &'static str {
    PRIORITIES.iter().find(|(id, _)| Some(*id) == priority).map_or("No priority", |(_, name)| name)
}

/// How many of the three bars are filled: High 3, Normal 2, Low 1.
pub fn priority_bars(priority: &str) -> usize {
    match priority {
        "high" => 3,
        "normal" => 2,
        "low" => 1,
        _ => 0,
    }
}

/// A due time, HH:MM on the 24-hour clock in the Macs' own time zone (API 88).
pub fn parse_time(time: &str) -> Option<NaiveTime> {
    if time.len() != 5 { return None; }
    NaiveTime::parse_from_str(time, "%H:%M").ok()
}
pub fn format_time(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}
/// "9:30 AM", or "09:30" on a 24-hour clock.
pub fn time_label(time: NaiveTime, twenty_four: bool) -> String {
    if twenty_four { time.format("%H:%M").to_string() } else { time.format("%-I:%M %p").to_string() }
}

pub fn parse(day: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

pub fn format(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

/// "Tue, Nov 17", as the quick picks show their day.
pub fn long_label(day: NaiveDate) -> String {
    day.format("%a, %b %-d").to_string()
}

/// The quick picks: Today, Tomorrow, End of this week (its Sunday) and In one week.
pub fn quick_picks(today: NaiveDate) -> Vec<(&'static str, NaiveDate)> {
    let to_sunday = 6 - today.weekday().num_days_from_monday() as i64;
    vec![
        ("Today", today),
        ("Tomorrow", today + Duration::days(1)),
        ("End of this week", today + Duration::days(to_sunday)),
        ("In one week", today + Duration::days(7)),
    ]
}

/// A month's days as weeks from Monday, with None before the 1st and after the last.
pub fn month_grid(year: i32, month: u32) -> Vec<[Option<NaiveDate>; 7]> {
    let Some(first) = NaiveDate::from_ymd_opt(year, month, 1) else { return vec![] };
    let lead = first.weekday().num_days_from_monday() as usize;
    let mut weeks = Vec::new();
    let mut week = [None; 7];
    let mut day = first;
    let mut slot = lead;
    while day.month() == month {
        week[slot] = Some(day);
        slot += 1;
        if slot == 7 {
            weeks.push(week);
            week = [None; 7];
            slot = 0;
        }
        day += Duration::days(1);
    }
    if slot > 0 {
        weeks.push(week);
    }
    weeks
}

/// The month after (`step` 1) or before (-1).
pub fn shift_month(year: i32, month: u32, step: i32) -> (i32, u32) {
    let index = year * 12 + month as i32 - 1 + step;
    (index.div_euclid(12), index.rem_euclid(12) as u32 + 1)
}

pub fn month_title(year: i32, month: u32) -> String {
    NaiveDate::from_ymd_opt(year, month, 1).map(|d| d.format("%B %Y").to_string()).unwrap_or_default()
}

pub const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Due {
    Overdue,
    Today,
    Later,
}

pub fn due_state(due: NaiveDate, today: NaiveDate) -> Due {
    if due < today {
        Due::Overdue
    } else if due == today {
        Due::Today
    } else {
        Due::Later
    }
}

/// The chip's words: Today, Tomorrow, Yesterday, or the day ("Nov 17", with
/// the year when it isn't this one).
pub fn chip_label(due: NaiveDate, today: NaiveDate) -> String {
    match (due - today).num_days() {
        0 => "Today".into(),
        1 => "Tomorrow".into(),
        -1 => "Yesterday".into(),
        _ if due.year() == today.year() => due.format("%b %-d").to_string(),
        _ => due.format("%b %-d, %Y").to_string(),
    }
}

/// A task with a due date, as the Scheduled view lists it beside the
/// schedules. It only shows when it's due: nothing here runs or sends.
#[derive(Clone, Debug, PartialEq)]
pub struct DueTask {
    pub id: String,
    pub title: String,
    pub due: NaiveDate,
    pub status: String,
    pub priority: Option<String>,
    /// Its project's id, when it has one, for the row's project name.
    pub project_id: Option<String>,
    /// Set on a sub-task: the parent's title, so the row says what it's part of.
    pub parent_title: Option<String>,
}

fn priority_rank(priority: Option<&str>) -> usize {
    PRIORITIES.iter().position(|(id, _)| Some(*id) == priority).unwrap_or(PRIORITIES.len())
}

/// The tasks from `tasks.list` that have a due date and aren't done, soonest
/// first, then the most pressing, then by title. A day that doesn't parse
/// is left out rather than guessed at.
pub fn due_tasks(items: &[serde_json::Value]) -> Vec<DueTask> {
    let titles: std::collections::HashMap<&str, &str> = items.iter().filter_map(|t| Some((t["id"].as_str()?, t["title"].as_str().unwrap_or("")))).collect();
    let mut out: Vec<DueTask> = items
        .iter()
        .filter_map(|t| {
            let status = t["status"].as_str().unwrap_or("todo");
            if status == "done" {
                return None;
            }
            let due = parse(t["dueDate"].as_str()?)?;
            let parent_title = t["parentId"].as_str().and_then(|p| titles.get(p)).map(|s| s.to_string());
            Some(DueTask {
                id: t["id"].as_str()?.to_string(),
                title: t["title"].as_str().unwrap_or("").to_string(),
                due,
                status: status.to_string(),
                priority: t["priority"].as_str().map(String::from),
                project_id: t["project"]["id"].as_str().map(String::from),
                parent_title,
            })
        })
        .collect();
    out.sort_by(|a, b| a.due.cmp(&b.due).then(priority_rank(a.priority.as_deref()).cmp(&priority_rank(b.priority.as_deref()))).then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())));
    out
}

/// How many are overdue or due today: the heading's count when it's folded.
pub fn due_now_count(tasks: &[DueTask], today: NaiveDate) -> usize {
    tasks.iter().filter(|t| t.due <= today).count()
}

/// The Scheduled page's line about tasks: "1 task overdue, 2 due today",
/// None when nothing is due yet.
pub fn due_summary(tasks: &[DueTask], today: NaiveDate) -> Option<String> {
    let overdue = tasks.iter().filter(|t| t.due < today).count();
    let now = tasks.iter().filter(|t| t.due == today).count();
    let noun = |n: usize| if n == 1 { "task" } else { "tasks" };
    match (overdue, now) {
        (0, 0) => None,
        (0, n) => Some(format!("{n} {} due today", noun(n))),
        (o, 0) => Some(format!("{o} {} overdue", noun(o))),
        (o, n) => Some(format!("{o} {} overdue, {n} due today", noun(o))),
    }
}

/// The Scheduled page's tiles for Tasks, after the schedules' own: every
/// task with a date, the ones due today and the ones overdue.
pub fn due_stats(tasks: &[DueTask], today: NaiveDate) -> Vec<crate::home::HomeStat> {
    use crate::home::HomeStat;
    let now = tasks.iter().filter(|t| t.due == today).count();
    let overdue = tasks.iter().filter(|t| t.due < today).count();
    vec![
        HomeStat::new(tasks.len(), if tasks.len() == 1 { "Task due" } else { "Tasks due" }, "Tasks with a due date that aren't done yet"),
        HomeStat::new(now, if now == 1 { "Task due today" } else { "Tasks due today" }, "Tasks whose due date is today"),
        HomeStat::new(overdue, if overdue == 1 { "Task overdue" } else { "Tasks overdue" }, "Tasks whose due date has passed and aren't done"),
    ]
}

/// True for the quick pick's Sunday, so tests can see the week ends there.
pub fn is_sunday(day: NaiveDate) -> bool {
    day.weekday() == Weekday::Sun
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        parse(s).unwrap()
    }

    #[test]
    fn quick_picks_end_the_week_on_sunday() {
        // 17 Nov 2026 is a Tuesday.
        let picks = quick_picks(d("2026-11-17"));
        assert_eq!(picks[0], ("Today", d("2026-11-17")));
        assert_eq!(picks[1], ("Tomorrow", d("2026-11-18")));
        assert_eq!(picks[2], ("End of this week", d("2026-11-22")));
        assert!(is_sunday(picks[2].1));
        assert_eq!(picks[3], ("In one week", d("2026-11-24")));
        // On a Sunday the week ends today.
        assert_eq!(quick_picks(d("2026-11-22"))[2].1, d("2026-11-22"));
        assert_eq!(long_label(d("2026-11-17")), "Tue, Nov 17");
    }

    #[test]
    fn the_month_starts_on_monday() {
        // November 2026 starts on a Sunday: six empty slots first.
        let weeks = month_grid(2026, 11);
        assert_eq!(weeks[0][..6], [None; 6]);
        assert_eq!(weeks[0][6], Some(d("2026-11-01")));
        assert_eq!(weeks.last().unwrap()[0], Some(d("2026-11-30")));
        assert_eq!(weeks.iter().flatten().flatten().count(), 30);
        assert_eq!(shift_month(2026, 12, 1), (2027, 1));
        assert_eq!(shift_month(2026, 1, -1), (2025, 12));
        assert_eq!(month_title(2026, 11), "November 2026");
    }

    #[test]
    fn due_tasks_are_soonest_first_and_skip_done() {
        use serde_json::json;
        let items = vec![
            json!({"id":"a","title":"Later one","status":"todo","dueDate":"2026-11-30"}),
            json!({"id":"b","title":"No date","status":"todo"}),
            json!({"id":"c","title":"Finished","status":"done","dueDate":"2026-11-10"}),
            json!({"id":"d","title":"beta","status":"doing","dueDate":"2026-11-17","priority":"low","project":{"runnerId":"r","id":"p1"}}),
            json!({"id":"e","title":"Alpha","status":"todo","dueDate":"2026-11-17","priority":"urgent","parentId":"a"}),
            json!({"id":"f","title":"Bad day","status":"todo","dueDate":"soon"}),
            json!({"id":"g","title":"Gamma","status":"todo","dueDate":"2026-11-17"}),
        ];
        let due = due_tasks(&items);
        let ids: Vec<&str> = due.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["e", "d", "g", "a"]);
        assert_eq!(due[0].parent_title.as_deref(), Some("Later one"));
        assert_eq!(due[1].project_id.as_deref(), Some("p1"));
        assert_eq!(due[1].status, "doing");
        assert_eq!(due_now_count(&due, d("2026-11-17")), 3);
        assert_eq!(due_now_count(&due, d("2026-11-16")), 0);
        assert_eq!(due_summary(&due, d("2026-11-17")).as_deref(), Some("3 tasks due today"));
        assert_eq!(due_summary(&due, d("2026-11-18")).as_deref(), Some("3 tasks overdue"));
        assert_eq!(due_summary(&due, d("2026-11-30")).as_deref(), Some("3 tasks overdue, 1 due today"));
        assert_eq!(due_summary(&due[3..], d("2026-12-01")).as_deref(), Some("1 task overdue"));
        assert_eq!(due_summary(&due, d("2026-11-01")), None);
        let tiles: Vec<(String, String)> = due_stats(&due, d("2026-11-17")).into_iter().map(|s| (s.value, s.label)).collect();
        assert_eq!(tiles, [("4".into(), "Tasks due".into()), ("3".into(), "Tasks due today".into()), ("0".into(), "Tasks overdue".into())]);
        assert_eq!(due_stats(&due[3..], d("2026-12-01"))[2].label, "Task overdue");
    }

    #[test]
    fn chips_say_how_soon() {
        let today = d("2026-11-17");
        assert_eq!(chip_label(today, today), "Today");
        assert_eq!(chip_label(d("2026-11-18"), today), "Tomorrow");
        assert_eq!(chip_label(d("2026-11-16"), today), "Yesterday");
        assert_eq!(chip_label(d("2026-11-30"), today), "Nov 30");
        assert_eq!(chip_label(d("2027-01-03"), today), "Jan 3, 2027");
        assert_eq!(due_state(d("2026-11-10"), today), Due::Overdue);
        assert_eq!(due_state(today, today), Due::Today);
        assert_eq!(priority_name(Some("high")), "High");
        assert_eq!(priority_name(None), "No priority");
        assert_eq!(priority_bars("low"), 1);
    }
}

#[cfg(test)]
mod due_time_tests {
    use super::*;
    #[test]
    fn due_times_are_hh_mm_and_read_both_clocks() {
        let t = parse_time("09:30").unwrap();
        assert_eq!(format_time(t), "09:30");
        assert_eq!(time_label(t, false), "9:30 AM");
        assert_eq!(time_label(parse_time("17:05").unwrap(), true), "17:05");
        for bad in ["9:30", "24:00", "09:60", "noon", "09:30:00"] {
            assert!(parse_time(bad).is_none(), "{bad}");
        }
    }
}
