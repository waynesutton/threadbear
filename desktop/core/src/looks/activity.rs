//! The Librarian page's Threadbear activity: a year of days shaded by how
//! many threads you started each day, as the profile page's card counts them
//! (runner/src/public-profile.ts, activityOf). Worked out on this machine from
//! the threads the app already has, so nothing is fetched or sent. It comes
//! out as GitHubContributions, so the same calendar draws both.

use super::github::{ContributionDay, GitHubContributions};
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

/// One year of threads started, by local calendar day (`threads` holds each
/// thread's local creation day and its folder). Threads in a private folder
/// aren't counted, as on the profile page. `home` expands a `~`.
pub fn thread_activity(year: i32, threads: &[(NaiveDate, String)], private_folders: &[String], home: &str) -> GitHubContributions {
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    for (day, cwd) in threads {
        if day.year() != year || is_private(cwd, private_folders, home) {
            continue;
        }
        *counts.entry(day.format("%Y-%m-%d").to_string()).or_default() += 1;
    }
    let most = counts.values().copied().max().unwrap_or(0);
    let days = counts.iter().map(|(date, &count)| ContributionDay { date: date.clone(), count, level: level(count, most) }).collect();
    GitHubContributions { year, total: counts.values().sum(), days }
}

/// 0 for none, then 1 to 4 by quarters of the busiest day, so a quiet year
/// still shades its busiest days darkest.
pub fn level(count: i64, most: i64) -> u8 {
    if count <= 0 || most <= 0 {
        return 0;
    }
    ((4. * count as f64 / most as f64).ceil() as i64).clamp(1, 4) as u8
}

/// The first year with a thread, for how far back the arrows go.
pub fn first_year(dates: &[NaiveDate]) -> Option<i32> {
    dates.iter().min().map(Datelike::year)
}

/// A folder in the list, or anywhere under one, with `~` for home, as the
/// runner's own check (runner/src/models/index.ts, isPrivate).
pub fn is_private(cwd: &str, folders: &[String], home: &str) -> bool {
    let at = standard(cwd, home);
    folders.iter().any(|folder| {
        let p = standard(folder, home);
        if p.is_empty() {
            return false;
        }
        let under = if p.ends_with('/') { p.clone() } else { format!("{p}/") };
        at == p || at.starts_with(&under)
    })
}

/// `~` expanded, `.`, `..` and doubled slashes taken out, and no trailing
/// slash, as NSString's standardizingPath leaves a path.
fn standard(path: &str, home: &str) -> String {
    let expanded = if path == "~" {
        home.to_owned()
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{}/{rest}", home.trim_end_matches('/'))
    } else {
        path.to_owned()
    };
    if expanded.is_empty() {
        return String::new();
    }
    let absolute = expanded.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in expanded.split('/') {
        match part {
            "" | "." => {}
            ".." if absolute => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn counts_threads_by_day_in_the_year_and_leaves_out_private_folders() {
        let threads = vec![
            (day("2026-11-04"), "/Users/w/repo".to_owned()),
            (day("2026-11-04"), "/Users/w/repo".to_owned()),
            (day("2026-11-05"), "/Users/w/notes".to_owned()),
            (day("2026-11-05"), "/Users/w/secret/deep".to_owned()),
            (day("2025-12-31"), "/Users/w/repo".to_owned()),
        ];
        let year = thread_activity(2026, &threads, &["/Users/w/secret".to_owned()], "/Users/w");
        assert_eq!(year.year, 2026);
        assert_eq!(year.total, 3);
        assert_eq!(year.days.iter().map(|d| (d.date.as_str(), d.count, d.level)).collect::<Vec<_>>(), [("2026-11-04", 2, 4), ("2026-11-05", 1, 2)]);
        // Drawn by the same calendar as GitHub's: every day of the year.
        assert_eq!(year.weeks().iter().flatten().flatten().count(), 365);
    }

    #[test]
    fn levels() {
        assert_eq!(level(0, 10), 0);
        assert_eq!(level(1, 10), 1);
        assert_eq!(level(5, 10), 2);
        assert_eq!(level(10, 10), 4);
        assert_eq!(level(1, 1), 4);
    }

    #[test]
    fn private_folders() {
        let home = "/Users/w";
        assert!(is_private("/a/b", &["/a/b".into()], home));
        assert!(is_private("/a/b/c", &["/a/b/".into()], home));
        assert!(!is_private("/a/bc", &["/a/b".into()], home));
        assert!(!is_private("/a/b", &["".into()], home));
        assert!(is_private("/Users/w/p/q", &["~/p".into()], home));
    }

    #[test]
    fn first_years() {
        assert_eq!(first_year(&[]), None);
        assert_eq!(first_year(&[day("2026-03-01"), day("2025-06-01")]), Some(2025));
    }
}
