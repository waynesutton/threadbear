//! Tasks' due dates and priorities: the quick picks, the month calendar and
//! the chip's words. Due dates are plain days (`YYYY-MM-DD`), no time or zone.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

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
