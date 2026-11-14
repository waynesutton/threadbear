//! One year of someone's public GitHub contributions, from the GitHub
//! Contributions API (github-contributions-api.jogruber.de), which reads the
//! calendar on their GitHub profile. No token, and only the user name is
//! sent. Threadbear activity comes out in the same shape, so one calendar
//! draws both.

use super::github_user_clean;
use chrono::{Datelike, NaiveDate};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributionDay {
    /// yyyy-MM-dd.
    pub date: String,
    pub count: i64,
    /// 0 (none) to 4 (most), as GitHub shades it.
    pub level: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubContributions {
    pub year: i32,
    pub total: i64,
    pub days: Vec<ContributionDay>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    Unreadable,
    /// The service's own error, such as an unknown user.
    Service(String),
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

impl GitHubContributions {
    pub fn url(user: &str, year: i32) -> Option<String> {
        let name = github_user_clean(user);
        (!name.is_empty()).then(|| format!("https://github-contributions-api.jogruber.de/v4/{name}?y={year}"))
    }

    /// What to say when a year couldn't be fetched. Only a real "no such
    /// user" says the name is wrong; the service timing out on GitHub
    /// ("GitHub: Gateway timeout.", 8 Nov) or any other trouble says to try
    /// again, as the name is likely fine.
    pub fn failure(status: u16, service_message: Option<&str>, user: &str) -> String {
        let message = service_message.unwrap_or_default().to_lowercase();
        if status == 404 || message.contains("not found") || message.contains("no user") || message.contains("does not exist") {
            return format!("GitHub has no user called {}.", github_user_clean(user));
        }
        "GitHub isn't answering right now. Try again in a few minutes.".to_owned()
    }

    pub fn parse(bytes: &[u8], year: i32) -> Result<GitHubContributions, ParseError> {
        let v: Value = serde_json::from_slice(bytes).map_err(|_| ParseError::Unreadable)?;
        // Swift reads an error only from an object of nothing but strings.
        if let Some(map) = v.as_object() {
            if map.values().all(Value::is_string) {
                if let Some(message) = map.get("error").and_then(Value::as_str) {
                    return Err(ParseError::Service(message.to_owned()));
                }
            }
        }
        let list = v["contributions"].as_array().ok_or(ParseError::Unreadable)?;
        let mut wire = Vec::with_capacity(list.len());
        for day in list {
            let (Some(date), Some(count), Some(level)) = (day["date"].as_str(), day["count"].as_i64(), day["level"].as_i64()) else {
                return Err(ParseError::Unreadable);
            };
            wire.push((date.to_owned(), count, level));
        }
        let prefix = format!("{year}-");
        let mut days: Vec<ContributionDay> = wire
            .into_iter()
            .filter(|(date, _, _)| date.starts_with(&prefix))
            .map(|(date, count, level)| ContributionDay { date, count: count.max(0), level: level.clamp(0, 4) as u8 })
            .collect();
        days.sort_by(|a, b| a.date.cmp(&b.date));
        let total = v["total"][year.to_string()].as_i64().unwrap_or_else(|| days.iter().map(|d| d.count).sum());
        Ok(GitHubContributions { year, total, days })
    }

    /// The calendar as GitHub draws it: a column per week, Sunday at the top,
    /// every day of the year in place (a day the service didn't send counts
    /// as none), and None before the 1st and after the 31st.
    pub fn weeks(&self) -> Vec<[Option<ContributionDay>; 7]> {
        let (Some(first), Some(next)) = (NaiveDate::from_ymd_opt(self.year, 1, 1), NaiveDate::from_ymd_opt(self.year + 1, 1, 1)) else { return Vec::new() };
        let by_date: std::collections::HashMap<&str, &ContributionDay> = self.days.iter().rev().map(|d| (d.date.as_str(), d)).collect();
        let mut weeks = Vec::new();
        let mut week: [Option<ContributionDay>; 7] = Default::default();
        let mut row = first.weekday().num_days_from_sunday() as usize;
        let mut date = first;
        while date < next {
            let key = date.format("%Y-%m-%d").to_string();
            week[row] = Some(by_date.get(key.as_str()).map_or_else(|| ContributionDay { date: key, count: 0, level: 0 }, |d| (*d).clone()));
            row += 1;
            if row == 7 {
                weeks.push(std::mem::take(&mut week));
                row = 0;
            }
            date = date.succ_opt().unwrap_or(next);
        }
        if row > 0 {
            weeks.push(week);
        }
        weeks
    }

    /// Each month's short name and the week column its 1st falls in.
    pub fn month_starts(&self) -> Vec<(&'static str, usize)> {
        let mut starts = Vec::new();
        for (index, week) in self.weeks().iter().enumerate() {
            for day in week.iter().flatten() {
                if day.date.ends_with("-01") {
                    let month: usize = day.date.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
                    starts.push((MONTHS[month.clamp(1, 12) - 1], index));
                }
            }
        }
        starts
    }
}

/// "Mon Nov 3" for 2025-11-03, as the Swift card's "EEE MMM d" says it.
pub fn day_title(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(day) => day.format("%a %b %-d").to_string(),
        Err(_) => date.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(GitHubContributions::url("@octocat", 2026).as_deref(), Some("https://github-contributions-api.jogruber.de/v4/octocat?y=2026"));
        assert_eq!(GitHubContributions::url("", 2026), None);
    }

    #[test]
    fn parses_the_service_and_its_errors() {
        let json = r#"{"total":{"2026":9},"contributions":[{"date":"2026-01-02","count":5,"level":2},{"date":"2026-01-01","count":4,"level":9},{"date":"2025-12-31","count":1,"level":1}]}"#;
        let year = GitHubContributions::parse(json.as_bytes(), 2026).unwrap();
        assert_eq!(year.total, 9);
        assert_eq!(year.days.iter().map(|d| d.date.as_str()).collect::<Vec<_>>(), ["2026-01-01", "2026-01-02"]);
        assert_eq!(year.days[0].level, 4, "levels are kept to 0...4");
        assert_eq!(GitHubContributions::parse(br#"{"error":"Not found"}"#, 2026), Err(ParseError::Service("Not found".into())));
        assert!(GitHubContributions::parse(b"<html>", 2026).is_err());
        let no_total = GitHubContributions::parse(br#"{"contributions":[{"date":"2026-03-01","count":2,"level":1}]}"#, 2026).unwrap();
        assert_eq!(no_total.total, 2, "the days add up when there's no total");
    }

    #[test]
    fn calendar_has_every_day_in_place() {
        // 1 Jan 2026 is a Thursday: four empty cells above it.
        let year = GitHubContributions { year: 2026, total: 3, days: vec![ContributionDay { date: "2026-01-01".into(), count: 3, level: 2 }] };
        let weeks = year.weeks();
        assert_eq!(weeks.len(), 53);
        assert!(weeks[0][..4].iter().all(Option::is_none));
        assert_eq!(weeks[0][4].as_ref().map(|d| (d.date.as_str(), d.count)), Some(("2026-01-01", 3)));
        assert_eq!(weeks.iter().flatten().flatten().count(), 365);
        assert_eq!(weeks.last().and_then(|w| w.iter().flatten().last()).map(|d| d.date.as_str()), Some("2026-12-31"));
        let months = year.month_starts();
        assert_eq!(months.len(), 12);
        assert_eq!(months[0], ("Jan", 0));
        assert_eq!(months[11].0, "Dec");
    }

    #[test]
    fn github_trouble_only_blames_the_name_when_it_is_the_name() {
        let later = "GitHub isn't answering right now. Try again in a few minutes.";
        assert_eq!(GitHubContributions::failure(504, Some("GitHub: Gateway timeout."), "waynesutton"), later);
        assert_eq!(GitHubContributions::failure(500, None, "waynesutton"), later);
        assert_eq!(GitHubContributions::failure(404, None, "nobody-here"), "GitHub has no user called nobody-here.");
        assert_eq!(GitHubContributions::failure(200, Some("User not found"), "x"), "GitHub has no user called x.");
    }

    #[test]
    fn day_titles() {
        assert_eq!(day_title("2025-11-03"), "Mon Nov 3");
        assert_eq!(day_title("nonsense"), "nonsense");
    }
}
