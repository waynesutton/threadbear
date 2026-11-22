// The Librarian page's two year calendars, a port of the Swift app's
// GitHubActivityCard and ThreadbearActivityCard: someone's public GitHub
// contributions a year at a time, from the GitHub Contributions API
// (github-contributions-api.jogruber.de, which reads the calendar on their
// GitHub profile; only the user name is sent, and no token), and a year of
// the threads you started, worked out from the threads the app already has
// (threads in a private folder aren't counted). A square a day, darker for
// more, grey in both appearances like the rest of the page; hovering a day
// says its count, and a GitHub day opens that day on the profile.
use crate::a11y::A11yRole as _;
use crate::app::Threadbear;
use crate::looks::{kit_button, tile_fill};
use crate::style::{self, px};
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{h_flex, v_flex, Sizable as _, StyledExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use threadbear_core::chrono::{Datelike as _, Local, TimeZone as _};
use threadbear_core::looks::activity::thread_activity;
use threadbear_core::looks::github::{ContributionDay, GitHubContributions, ParseError};
use threadbear_core::looks::github_user_clean;

/// The squares' pitch at most and at least, the labels' room, the months'.
const LARGEST: f32 = 13.;
const SMALLEST: f32 = 8.;
const LABELS: f32 = 30.;
const MONTHS: f32 = 18.;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Fetched {
    Loading,
    Loaded(GitHubContributions),
    Failed(String),
}

/// What the cards hold: GitHub's years, fetched once and kept (the current
/// year again after an hour, past years never), and each card's year and
/// hovered day.
#[derive(Default)]
pub(crate) struct CardsState {
    years: HashMap<String, Fetched>,
    /// Years showing a saved copy because the last fetch failed.
    stale: HashSet<String>,
    fetching: HashSet<String>,
    fetched_at: HashMap<String, std::time::Instant>,
    pub(crate) github_year: Option<i32>,
    pub(crate) activity_year: Option<i32>,
    hovered: Option<(&'static str, ContributionDay)>,
    widths: HashMap<&'static str, Rc<Cell<f32>>>,
}

fn this_year() -> i32 {
    Local::now().year()
}

fn year_key(user: &str, year: i32) -> String {
    format!("{}/{year}", github_user_clean(user).to_lowercase())
}

/// The last answer for a year, kept in `looks/` so the card has something
/// to show at launch and while GitHub or the service is down.
fn cache_file(user: &str, year: i32) -> std::path::PathBuf {
    crate::looks::folder().join(format!(
        "github-{}-{year}.json",
        github_user_clean(user).to_lowercase()
    ))
}

/// Whether showing the card should fetch, independent of its network call.
fn year_needs_fetch(
    state: Option<&Fetched>,
    year: i32,
    current_year: i32,
    last_attempt: Option<std::time::Instant>,
    now: std::time::Instant,
) -> bool {
    match state {
        None => true,
        Some(Fetched::Loaded(_)) if year == current_year => last_attempt.is_none_or(|at| {
            now.saturating_duration_since(at) >= std::time::Duration::from_secs(3600)
        }),
        _ => false,
    }
}

/// One fetch, off the main thread: the year, or what to say.
fn fetch(user: &str, year: i32) -> (Result<GitHubContributions, String>, Option<Vec<u8>>) {
    let Some(url) = GitHubContributions::url(user, year) else {
        return (Err("That isn't a GitHub user name.".into()), None);
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .http_status_as_error(false)
        .user_agent("threadbear-desktop")
        .build()
        .into();
    let mut response = match agent.get(&url).header("Accept", "application/json").call() {
        Ok(r) => r,
        Err(_) => {
            return (
                Err("Couldn't reach GitHub. Check you're online and try again.".into()),
                None,
            )
        }
    };
    let status = response.status().as_u16();
    let Ok(body) = response.body_mut().read_to_vec() else {
        return (Err(GitHubContributions::failure(status, None, user)), None);
    };
    match GitHubContributions::parse(&body, year) {
        Ok(c) if status < 400 => (Ok(c), Some(body)),
        Ok(_) => (Err(GitHubContributions::failure(status, None, user)), None),
        Err(ParseError::Service(message)) => (
            Err(GitHubContributions::failure(status, Some(&message), user)),
            None,
        ),
        Err(ParseError::Unreadable) => (
            Err(if status == 200 {
                "GitHub's answer couldn't be read.".into()
            } else {
                GitHubContributions::failure(status, None, user)
            }),
            None,
        ),
    }
}

/// Grey from faint to full, the page's own ink in light and dark.
fn shade(level: u8, cx: &App) -> Hsla {
    const OPACITY: [f32; 5] = [0.07, 0.22, 0.42, 0.66, 0.92];
    style::primary_opacity(cx, OPACITY[usize::from(level.min(4))])
}

impl Threadbear {
    /// Loads a year unless it's held and fresh enough: a past year once, the
    /// current one again after an hour. `force` is the refresh button. A
    /// refresh keeps what's showing until the answer comes.
    pub(crate) fn load_github_year(
        &mut self,
        user: &str,
        year: i32,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let user = github_user_clean(user);
        if user.is_empty() {
            return;
        }
        let k = year_key(&user, year);
        if self.cards.fetching.contains(&k) {
            return;
        }
        if !self.cards.years.contains_key(&k) {
            if let Some(saved) = std::fs::read(cache_file(&user, year))
                .ok()
                .and_then(|b| GitHubContributions::parse(&b, year).ok())
            {
                self.cards.years.insert(k.clone(), Fetched::Loaded(saved));
            }
        }
        if !force && matches!(self.cards.years.get(&k), Some(Fetched::Loaded(_))) {
            let fresh = self
                .cards
                .fetched_at
                .get(&k)
                .is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(3600));
            if fresh || (year < this_year() && std::fs::metadata(cache_file(&user, year)).is_ok()) {
                return;
            }
        }
        if !matches!(self.cards.years.get(&k), Some(Fetched::Loaded(_))) {
            self.cards.years.insert(k.clone(), Fetched::Loading);
        }
        self.cards.fetching.insert(k.clone());
        // A failed refresh keeps the calendar, with an hour before another
        // automatic attempt. Refresh still retries immediately.
        self.cards
            .fetched_at
            .insert(k.clone(), std::time::Instant::now());
        cx.notify();
        cx.spawn(async move |this, cx| {
            let who = user.clone();
            let (result, body) = cx
                .background_executor()
                .spawn(async move { fetch(&who, year) })
                .await;
            this.update(cx, |this, cx| {
                this.cards.fetching.remove(&k);
                match result {
                    Err(message) => {
                        if matches!(this.cards.years.get(&k), Some(Fetched::Loaded(_))) {
                            this.cards.stale.insert(k);
                        } else {
                            this.cards.years.insert(k, Fetched::Failed(message));
                        }
                    }
                    Ok(year_of) => {
                        this.cards.years.insert(k.clone(), Fetched::Loaded(year_of));
                        this.cards
                            .fetched_at
                            .insert(k.clone(), std::time::Instant::now());
                        this.cards.stale.remove(&k);
                        if let Some(body) = body {
                            let _ = std::fs::create_dir_all(crate::looks::folder());
                            let _ = threadbear_core::private_file::write(
                                &cache_file(&user, year),
                                &body,
                            );
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The GitHub card: its title and the year's total, the years' arrows and
    /// refresh, the calendar, and what the hovered day had.
    pub(crate) fn render_github_card(
        &mut self,
        user: &str,
        on_picture: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let name = github_user_clean(user);
        let year = self.cards.github_year.unwrap_or_else(this_year);
        let k = year_key(&name, year);
        if !self.cards.fetching.contains(&k)
            && year_needs_fetch(
                self.cards.years.get(&k),
                year,
                this_year(),
                self.cards.fetched_at.get(&k).copied(),
                std::time::Instant::now(),
            )
        {
            // Asked for once it's drawn, as the Swift card's `.task`.
            let me = cx.entity().downgrade();
            let user = name.clone();
            cx.defer(move |cx| {
                me.update(cx, |this, cx| this.load_github_year(&user, year, false, cx))
                    .ok();
            });
        }
        let state = self.cards.years.get(&k).cloned();
        let subtitle = match &state {
            Some(Fetched::Loaded(c)) => format!(
                "{} {} in {year}",
                thousands(c.total),
                if c.total == 1 {
                    "contribution"
                } else {
                    "contributions"
                }
            ),
            Some(Fetched::Failed(message)) => message.clone(),
            _ => format!("Loading @{name}'s {year}…"),
        };
        let fetching = self.cards.fetching.contains(&k);
        let refresh_user = name.clone();
        let stepper = h_flex()
            .gap(px(6.))
            .items_center()
            .child(kit_button(
                "github-year-before",
                "‹",
                None,
                "The year before",
                year <= 2008,
                false,
                cx.listener(move |this, _, _, cx| {
                    this.cards.github_year = Some(year - 1);
                    cx.notify();
                }),
            ))
            .child(
                div()
                    .min_w(px(44.))
                    .flex()
                    .justify_center()
                    .text_size(px(15.))
                    .font_medium()
                    .child(year.to_string()),
            )
            .child(kit_button(
                "github-year-after",
                "›",
                None,
                "The year after",
                year >= this_year(),
                false,
                cx.listener(move |this, _, _, cx| {
                    this.cards.github_year = Some(year + 1);
                    cx.notify();
                }),
            ))
            .child(kit_button(
                "github-refresh",
                "Refresh",
                None,
                "Fetch this year from GitHub again",
                fetching,
                false,
                cx.listener(move |this, _, _, cx| {
                    this.load_github_year(&refresh_user, year, true, cx)
                }),
            ));
        let body: AnyElement = match &state {
            Some(Fetched::Loaded(c)) => {
                let open_user = name.clone();
                self.contribution_grid(
                    "github",
                    c,
                    move |day, cx| {
                        let url = format!(
                            "https://github.com/{open_user}?tab=overview&from={}&to={}",
                            day.date, day.date
                        );
                        crate::open_gate::open_web(&url, cx);
                    },
                    cx,
                )
            }
            Some(Fetched::Failed(_)) => {
                let again = name.clone();
                div()
                    .h(px(MONTHS + 7. * LARGEST))
                    .flex()
                    .items_center()
                    .child(kit_button(
                        "github-try-again",
                        "Try again",
                        None,
                        "Fetch this year from GitHub again",
                        false,
                        false,
                        cx.listener(move |this, _, _, cx| {
                            this.load_github_year(&again, year, true, cx)
                        }),
                    ))
                    .into_any_element()
            }
            _ => div()
                .h(px(MONTHS + 7. * LARGEST))
                .flex()
                .items_center()
                .justify_center()
                .child(Spinner::new().with_size(px(14.)))
                .into_any_element(),
        };
        let readout = match self
            .cards
            .hovered
            .as_ref()
            .filter(|(card, _)| *card == "github")
        {
            Some((_, day)) => {
                let when = threadbear_core::looks::github::day_title(&day.date);
                match day.count {
                    0 => format!("No contributions on {when}"),
                    1 => format!("1 contribution on {when}"),
                    n => format!("{n} contributions on {when}"),
                }
            }
            None if self.cards.stale.contains(&k) => {
                "GitHub isn't answering right now, so this is the last copy".to_owned()
            }
            None => format!("@{name}'s public contributions"),
        };
        calendar_card(
            "github-card",
            "GitHub activity",
            subtitle,
            stepper.into_any_element(),
            body,
            readout,
            on_picture,
            cx,
        )
    }

    /// The Threadbear card: a year of the threads you started, by this
    /// Mac's calendar day, from the first year with a thread to this one.
    pub(crate) fn render_activity_card(
        &mut self,
        on_picture: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let home = crate::files::home_dir();
        let private: Vec<String> = self
            .runner_settings
            .as_ref()
            .and_then(|s| s["models"]["privateFolders"].as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let threads: Vec<(threadbear_core::chrono::NaiveDate, String)> = self
            .threads
            .values()
            .filter_map(|t| {
                let ms = t
                    .created_at
                    .as_deref()
                    .and_then(threadbear_core::inbox::parse_ms)?;
                let local = Local.timestamp_millis_opt(ms).single()?;
                Some((local.date_naive(), t.cwd.clone()))
            })
            .collect();
        let now = this_year();
        let first = threads
            .iter()
            .map(|(d, _)| d.year())
            .min()
            .unwrap_or(now)
            .min(now);
        let year = self.cards.activity_year.unwrap_or(now);
        let activity = thread_activity(year, &threads, &private, &home);
        let subtitle = format!(
            "{} {} started in {year}",
            thousands(activity.total),
            if activity.total == 1 {
                "thread"
            } else {
                "threads"
            }
        );
        let stepper = h_flex()
            .gap(px(6.))
            .items_center()
            .child(kit_button(
                "activity-year-before",
                "‹",
                None,
                "The year before",
                year <= first,
                false,
                cx.listener(move |this, _, _, cx| {
                    this.cards.activity_year = Some(year - 1);
                    cx.notify();
                }),
            ))
            .child(
                div()
                    .min_w(px(44.))
                    .flex()
                    .justify_center()
                    .text_size(px(15.))
                    .font_medium()
                    .child(year.to_string()),
            )
            .child(kit_button(
                "activity-year-after",
                "›",
                None,
                "The year after",
                year >= now,
                false,
                cx.listener(move |this, _, _, cx| {
                    this.cards.activity_year = Some(year + 1);
                    cx.notify();
                }),
            ));
        let body = self.contribution_grid("activity", &activity, |_, _| {}, cx);
        let readout = match self
            .cards
            .hovered
            .as_ref()
            .filter(|(card, _)| *card == "activity")
        {
            Some((_, day)) => {
                let when = threadbear_core::looks::github::day_title(&day.date);
                match day.count {
                    0 => format!("No threads on {when}"),
                    1 => format!("1 thread on {when}"),
                    n => format!("{n} threads on {when}"),
                }
            }
            None => "Threads in a private folder aren't counted".to_owned(),
        };
        calendar_card(
            "activity-card",
            "Threadbear activity",
            subtitle,
            stepper.into_any_element(),
            body,
            readout,
            on_picture,
            cx,
        )
    }

    /// The squares (`ContributionGrid`), with month names above and Mon,
    /// Wed and Fri beside, as GitHub has them. They shrink to fit the card,
    /// down to a size still easy to hover; past that the grid scrolls
    /// horizontally so every day remains reachable.
    fn contribution_grid(
        &mut self,
        card: &'static str,
        year: &GitHubContributions,
        open: impl Fn(&ContributionDay, &mut App) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let weeks = year.weeks();
        let starts = year.month_starts();
        let measured = self.cards.widths.entry(card).or_default().clone();
        let width = measured.get();
        let fits = if width > 0. {
            (width - LABELS) / weeks.len().max(1) as f32
        } else {
            LARGEST
        };
        let pitch = fits.clamp(SMALLEST, LARGEST);
        let gap = (pitch * 0.2).max(1.5);
        let side = pitch - gap;
        // Keep every week reachable, even in a narrow panel.
        let skip = 0;
        let secondary = style::secondary_text(cx);
        let hovered = self
            .cards
            .hovered
            .as_ref()
            .filter(|(c, _)| *c == card)
            .map(|(_, d)| d.date.clone());
        let open = Rc::new(open);
        let mut grid = div()
            .relative()
            .h(px(MONTHS + 7. * pitch))
            .w(px(LABELS + (weeks.len() - skip) as f32 * pitch));
        for (row, name) in [(1, "Mon"), (3, "Wed"), (5, "Fri")] {
            grid = grid.child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(MONTHS + row as f32 * pitch + side / 2. - 6.))
                    .text_size(px(10.))
                    .line_height(px(12.))
                    .text_color(secondary)
                    .child(name),
            );
        }
        for (month, week) in &starts {
            if *week >= skip {
                grid = grid.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(LABELS + (*week - skip) as f32 * pitch))
                        .text_size(px(10.))
                        .line_height(px(12.))
                        .text_color(secondary)
                        .child(*month),
                );
            }
        }
        let ink = style::primary_opacity(cx, 0.9);
        for (column, week) in weeks.iter().enumerate().skip(skip) {
            for (row, day) in week.iter().enumerate() {
                let Some(day) = day else { continue };
                let lit = hovered.as_deref() == Some(day.date.as_str());
                let (enter, click) = (day.clone(), day.clone());
                let open = open.clone();
                grid = grid.child(
                    div()
                        .id(SharedString::from(format!("{card}-{}", day.date)))
                        .absolute()
                        .left(px(LABELS + (column - skip) as f32 * pitch))
                        .top(px(MONTHS + row as f32 * pitch))
                        .size(px(side))
                        .rounded(px(2.))
                        .bg(shade(day.level, cx))
                        .tip(format!("{} on {}", day.count, day.date))
                        .when(lit, |d| d.border_1().border_color(ink))
                        .on_hover(cx.listener(move |this, over: &bool, _, cx| {
                            if *over {
                                this.cards.hovered = Some((card, enter.clone()));
                            } else if this
                                .cards
                                .hovered
                                .as_ref()
                                .is_some_and(|(c, d)| *c == card && d.date == enter.date)
                            {
                                this.cards.hovered = None;
                            }
                            cx.notify();
                        }))
                        .role(Role::Button)
                        .on_click(move |_, _, cx| open(&click, cx)),
                );
            }
        }
        let notify = cx.entity().downgrade();
        let unit = if card == "github" {
            ("contribution", "contributions")
        } else {
            ("thread", "threads")
        };
        let label = format!(
            "{} {} in {}",
            year.total,
            if year.total == 1 { unit.0 } else { unit.1 },
            year.year
        );
        div()
            .id(SharedString::from(format!("{card}-grid")))
            .test_support()
            .a11y_role(Role::GenericContainer)
            .aria_label(label)
            .relative()
            .w_full()
            .overflow_x_scroll()
            .child(grid)
            .child(
                canvas(
                    move |bounds, _, cx| {
                        let now = f32::from(bounds.size.width) / style::zoomed(1.);
                        if (measured.get() - now).abs() > 0.5 {
                            measured.set(now);
                            let notify = notify;
                            cx.defer(move |cx| {
                                notify.update(cx, |_, cx| cx.notify()).ok();
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }
}

/// A card's frame: the heading and its stepper, the calendar, then the
/// readout and Less to More.
#[allow(
    clippy::too_many_arguments,
    reason = "the parts of one row or card, drawn in one place"
)]
fn calendar_card(
    id: &'static str,
    title: &'static str,
    subtitle: String,
    stepper: AnyElement,
    body: AnyElement,
    readout: String,
    on_picture: bool,
    cx: &App,
) -> AnyElement {
    let secondary = style::secondary_text(cx);
    let mut legend = h_flex()
        .flex_none()
        .gap(px(4.))
        .items_center()
        .text_size(px(11.))
        .text_color(secondary)
        .child("Less");
    for level in 0..5u8 {
        legend = legend.child(div().size(px(10.)).rounded(px(2.)).bg(shade(level, cx)));
    }
    legend = legend.child("More");
    v_flex()
        .id(id)
        .w_full()
        .p(px(16.))
        .gap(px(12.))
        .rounded(px(12.))
        .bg(tile_fill(on_picture, cx))
        .child(
            h_flex()
                .w_full()
                .flex_wrap()
                .items_start()
                .gap(px(12.))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w(px(180.))
                        .gap(px(3.))
                        .child(
                            div()
                                .text_size(px(15.))
                                .line_height(px(19.))
                                .font_semibold()
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(style::size::LABEL))
                                .line_height(px(16.))
                                .text_color(secondary)
                                .child(subtitle),
                        ),
                )
                .child(stepper),
        )
        .child(body)
        .child(
            h_flex()
                .w_full()
                .flex_wrap()
                .gap(px(6.))
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .min_w(px(160.))
                        .truncate()
                        .text_size(px(style::size::LABEL))
                        .text_color(secondary)
                        .child(readout),
                )
                .child(legend),
        )
        .into_any_element()
}

/// 1,234, as Swift's `formatted()` puts it.
fn thousands(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn current_calendar_refreshes_after_an_hour_without_retrying_errors_on_each_draw() {
        let now = std::time::Instant::now();
        let loaded = super::Fetched::Loaded(threadbear_core::looks::github::GitHubContributions {
            year: 2026,
            total: 0,
            days: vec![],
        });
        assert!(super::year_needs_fetch(None, 2026, 2026, None, now));
        assert!(!super::year_needs_fetch(
            Some(&loaded),
            2026,
            2026,
            Some(now),
            now
        ));
        let later = now + std::time::Duration::from_secs(3601);
        assert!(super::year_needs_fetch(
            Some(&loaded),
            2026,
            2026,
            Some(now),
            later
        ));
        assert!(!super::year_needs_fetch(
            Some(&loaded),
            2025,
            2026,
            Some(now),
            later
        ));
        assert!(!super::year_needs_fetch(
            Some(&super::Fetched::Failed("offline".into())),
            2026,
            2026,
            None,
            later
        ));
        assert!(!super::year_needs_fetch(
            Some(&super::Fetched::Loading),
            2026,
            2026,
            None,
            later
        ));
    }

    #[test]
    fn thousands_has_commas() {
        assert_eq!(super::thousands(0), "0");
        assert_eq!(super::thousands(999), "999");
        assert_eq!(super::thousands(1234), "1,234");
        assert_eq!(super::thousands(1234567), "1,234,567");
    }
}
