// The Librarian and Scheduled pages laid out as the user set them (Settings >
// General > Look's Customize switches), a port of the Swift app's
// CustomizedPage, LibrarianHome's customized page, ScheduledHome's, and the
// cards only a customized page has: the greeting, the quote of the day, and
// the GitHub and Threadbear activity calendars. Customize at the top right
// opens the page's panel beside it (looks_editor.rs), which changes the page
// as you go.
use crate::a11y::A11yRole as _;
use crate::app::Threadbear;
use crate::home_page::{home_stat_tile_on, home_status_line, home_title_parts, tile_fill, Art};
use crate::looks::{self, Slot};
use crate::shell::SidebarMode;
use crate::style;
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{h_flex, v_flex, Sizable as _, StyledExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
// The zoom-aware px (style.rs), in place of GPUI's.
use crate::style::px;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use threadbear_core::chrono::{Datelike as _, Local, Timelike as _};
use threadbear_core::home::HomeStat;
use threadbear_core::looks::{self as rules, AboutPart, Contributions, LibrarianBlock, Picture, Place, ScheduledBlock};

/// A year of someone's GitHub contributions, as fetched (`GitHubActivity`).
#[derive(Clone, Debug)]
pub(crate) enum Fetched {
    Loading,
    Loaded(Contributions),
    Failed(String),
}

/// Everyone's years fetched so far, one copy, so moving between pages
/// doesn't fetch again. The current year is fetched again after an hour;
/// past years, which don't change, never.
#[derive(Default)]
pub(crate) struct GitHubActivity {
    states: HashMap<String, Fetched>,
    /// Showing a saved or earlier copy because the last fetch failed.
    stale: HashSet<String>,
    fetched_at: HashMap<String, std::time::SystemTime>,
    fetching: HashSet<String>,
}

fn key(user: &str, year: i32) -> String {
    format!("{}/{year}", rules::github_user(user).to_lowercase())
}

/// The last answer for a year, kept in `looks/` so the card has something
/// to show at launch and while GitHub or the service is down.
fn cache_file(user: &str, year: i32) -> std::path::PathBuf {
    looks::folder().join(format!("github-{}-{year}.json", rules::github_user(user).to_lowercase()))
}

/// One GET, off the main thread: the status and the body.
fn fetch(url: &str) -> Result<(u16, Vec<u8>), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(20))).http_status_as_error(false).user_agent("threadbear-desktop").build().into();
    let mut response = agent.get(url).header("Accept", "application/json").call().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let body = response.body_mut().with_config().limit(4 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())?;
    Ok((status, body))
}

/// The line under the Librarian page's title.
const LIBRARIAN_LINE: &str = "Keeps your projects tidy as your threads go.";

impl Threadbear {
    /// Fetches a year unless it's fresh enough (`GitHubActivity.load`).
    pub(crate) fn load_github_year(&mut self, user: &str, year: i32, force: bool, cx: &mut Context<Self>) {
        let k = key(user, year);
        let Some(url) = rules::github_url(user, year) else { return };
        let gh = &mut self.looks.github;
        if gh.fetching.contains(&k) {
            return;
        }
        if !gh.states.contains_key(&k) {
            let file = cache_file(user, year);
            if let Some(saved) = std::fs::read(&file).ok().and_then(|d| Contributions::parse(&d, year).ok()) {
                gh.states.insert(k.clone(), Fetched::Loaded(saved));
                if let Ok(at) = std::fs::metadata(&file).and_then(|m| m.modified()) {
                    gh.fetched_at.insert(k.clone(), at);
                }
            }
        }
        // A year that failed waits for Try again, rather than asking again
        // on every redraw.
        if !force && matches!(gh.states.get(&k), Some(Fetched::Failed(_) | Fetched::Loading)) {
            return;
        }
        if !force && matches!(gh.states.get(&k), Some(Fetched::Loaded(_))) {
            let this_year = Local::now().year();
            if year < this_year && gh.fetched_at.contains_key(&k) {
                return;
            }
            if gh.fetched_at.get(&k).and_then(|at| at.elapsed().ok()).is_some_and(|age| age < Duration::from_secs(3600)) {
                return;
            }
        }
        // A refresh keeps what's showing until the answer comes back.
        if !matches!(gh.states.get(&k), Some(Fetched::Loaded(_))) {
            gh.states.insert(k.clone(), Fetched::Loading);
        }
        gh.fetching.insert(k.clone());
        cx.notify();
        let user = user.to_owned();
        let job = cx.background_spawn(async move { fetch(&url) });
        cx.spawn(async move |this, cx| {
            let got = job.await;
            this.update(cx, |this, cx| {
                let gh = &mut this.looks.github;
                gh.fetching.remove(&k);
                let result = match got {
                    Ok((status, body)) => match Contributions::parse(&body, year) {
                        Ok(c) => {
                            let file = cache_file(&user, year);
                            let _ = std::fs::create_dir_all(looks::folder());
                            let _ = std::fs::write(file, &body);
                            Fetched::Loaded(c)
                        }
                        Err(rules::ParseError::Service(m)) => Fetched::Failed(rules::github_failure(status, Some(&m), &user)),
                        Err(rules::ParseError::Unreadable) => Fetched::Failed(if status == 200 { "GitHub's answer couldn't be read.".into() } else { rules::github_failure(status, None, &user) }),
                    },
                    Err(_) => Fetched::Failed("Couldn't reach GitHub. Check you're online and try again.".into()),
                };
                match result {
                    Fetched::Failed(_) if matches!(gh.states.get(&k), Some(Fetched::Loaded(_))) => {
                        gh.stale.insert(k);
                    }
                    Fetched::Loaded(c) => {
                        gh.fetched_at.insert(k.clone(), std::time::SystemTime::now());
                        gh.stale.remove(&k);
                        gh.states.insert(k, Fetched::Loaded(c));
                    }
                    other => {
                        gh.states.insert(k, other);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The greeting's clock and the quote's day move on by themselves while
    /// a customized Librarian page is showing, every half minute.
    fn keep_ticking(&mut self, cx: &mut Context<Self>) {
        if self.looks.ticking.is_some() {
            return;
        }
        self.looks.ticking = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(30)).await;
            let going = this
                .update(cx, |this, cx| {
                    let showing = this.home_page() == Some(SidebarMode::Librarian) && this.librarian_customizable();
                    if showing {
                        cx.notify();
                    } else {
                        this.looks.ticking = None;
                    }
                    showing
                })
                .unwrap_or(false);
            if !going {
                break;
            }
        }));
    }

    /// `CustomizedPage`: the parts in a scrolling column over the background,
    /// Customize at the top right, and the panel beside it while it's open.
    /// With `pins_places`, middle and bottom are where the parts sit in the
    /// window; without, only the order they stack in.
    fn render_customized_page(&mut self, slot: Slot, backdrop: Option<AnyElement>, zones: Vec<(Place, Vec<AnyElement>)>, pins_places: bool, max_width: f32, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let editing = self.looks.editing == Some(slot);
        let gap = 32.;
        let has = |place: Place| zones.iter().any(|(p, parts)| *p == place && !parts.is_empty());
        let (top, middle, bottom) = (has(Place::Top), has(Place::Middle), has(Place::Bottom));
        let column: AnyElement = if !(top || middle || bottom) {
            div().text_size(px(style::size::LABEL)).text_color(style::secondary_text(cx)).child("Everything on this page is hidden. Customize puts it back.").into_any_element()
        } else if pins_places {
            // Top at the top, middle in the space between, bottom on the
            // window's bottom edge; a page too tall just stacks and scrolls.
            // The gap only between places that have something.
            let mut col = v_flex().w_full().flex_1();
            for (place, parts) in zones {
                if place == Place::Middle {
                    col = col.child(div().flex_1().min_h(px(if top && (middle || bottom) { gap } else { 0. })));
                } else if place == Place::Bottom {
                    col = col.child(div().flex_1().min_h(px(if middle && bottom { gap } else { 0. })));
                }
                if !parts.is_empty() {
                    col = col.child(v_flex().w_full().gap(px(gap)).children(parts));
                }
            }
            col.into_any_element()
        } else {
            v_flex().w_full().gap(px(gap)).children(zones.into_iter().flat_map(|(_, parts)| parts)).into_any_element()
        };
        let customize = (!editing).then(|| {
            div().id(SharedString::from(format!("look-customize-{slot:?}"))).test_support().a11y_role(Role::GenericContainer).aria_label("Change the picture, the background, and where each part of this page sits").absolute().top(px(14.)).right(px(14.)).child(
                Button::new(SharedString::from(format!("look-customize-button-{slot:?}")))
                    .label("Customize")
                    .icon(crate::icons::symbol("paintbrush"))
                    .small()
                    .secondary()
                    .tooltip("Change the picture, the background, and where each part of this page sits")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.looks.editing = Some(slot);
                        cx.notify();
                    })),
            )
        });
        let page = div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .children(backdrop)
            .child(
                div().id(SharedString::from(format!("look-page-{slot:?}"))).absolute().inset_0().overflow_y_scroll().child(
                    h_flex().w_full().min_h_full().justify_center().child(
                        v_flex()
                            .w_full()
                            .min_h_full()
                            .max_w(px(max_width + 64.))
                            .px(px(32.))
                            // Clear of the Customize button.
                            .pt(px(56.))
                            .pb(px(48.))
                            .child(column),
                    ),
                ),
            )
            .children(customize);
        let editor = editing.then(|| {
            let panel = self.render_look_editor(slot, window, cx);
            div().flex_none().w(px(320.)).h_full().border_l_1().border_color(style::hairline(cx)).bg(gpui_kit::component::ActiveTheme::theme(&**cx).background).child(panel)
        });
        h_flex().size_full().child(page).children(editor).into_any_element()
    }

    /// The Librarian page as the user set it (`CustomizedLibrarianPage`).
    pub(crate) fn render_customized_librarian(&mut self, stats: &[HomeStat], window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = self.librarian_look();
        if look.greeting_shown || look.quote_shown {
            self.keep_ticking(cx);
        }
        let on_picture = looks::path(look.background.as_deref()).is_some();
        let backdrop = looks::backdrop(Slot::Librarian, look.background.as_deref(), look.background_dim, look.background_focus, cx);
        let layout = look.layout();
        let mut zones = Vec::new();
        for place in Place::ALL {
            let mut parts = Vec::new();
            for block in layout.iter().copied().filter(|b| look.place(*b) == place) {
                if let Some(part) = self.librarian_part(block, &look, stats, on_picture, cx) {
                    parts.push(part);
                }
            }
            zones.push((place, parts));
        }
        self.render_customized_page(Slot::Librarian, backdrop, zones, false, 1000., window, cx)
    }

    fn librarian_part(&mut self, block: LibrarianBlock, look: &rules::LibrarianLook, stats: &[HomeStat], on_picture: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let narrow = |e: AnyElement| div().max_w(px(620.)).child(e).into_any_element();
        let wide = |e: AnyElement| div().max_w(px(760.)).child(e).into_any_element();
        Some(match block {
            LibrarianBlock::Greeting => narrow(greeting(&look.greeting_name, look.greeting_clock, cx)),
            LibrarianBlock::Quote => narrow(self.render_quote(look, on_picture, cx)?),
            LibrarianBlock::Picture => self.look_picture(Art::Librarian, look.picture, look.picture_file.as_deref(), look.picture_size, cx)?,
            LibrarianBlock::About => {
                let status = self.librarian.status.clone();
                let title = look.shows_part(AboutPart::Title).then_some("Librarian");
                let line = look.shows_part(AboutPart::Line).then_some(LIBRARIAN_LINE);
                let mut about = v_flex().gap(px(12.));
                if title.is_some() || line.is_some() {
                    about = about.child(home_title_parts(title, line, crate::librarian::MORE, cx));
                }
                if let (Some(status), true) = (status, look.shows_part(AboutPart::Status)) {
                    about = about.child(home_status_line(&threadbear_core::librarian::state_text(&status), status.state == threadbear_core::librarian::State::Working, cx));
                }
                narrow(about.into_any_element())
            }
            LibrarianBlock::Numbers => numbers(stats, &rules::LIBRARIAN_NUMBERS, &look.hidden_numbers, on_picture, cx)?,
            LibrarianBlock::Activity => wide(self.render_thread_activity(on_picture, cx)),
            LibrarianBlock::Github => wide(self.render_github_activity(&look.github_user, on_picture, cx)),
            LibrarianBlock::Settings => {
                let status = self.librarian.status.clone()?;
                narrow(self.render_librarian_settings(&status, cx))
            }
        })
    }

    /// The Scheduled page as the user set it (`CustomizedScheduledPage`):
    /// top, middle and bottom are real places in the window here.
    pub(crate) fn render_customized_scheduled(&mut self, about: AnyElement, stats: &[HomeStat], window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let look = self.scheduled_look();
        let on_picture = looks::path(look.background.as_deref()).is_some();
        let backdrop = looks::backdrop(Slot::Scheduled, look.background.as_deref(), look.background_dim, look.background_focus, cx);
        let layout = look.layout();
        let mut about = Some(about);
        let mut zones = Vec::new();
        for place in Place::ALL {
            let mut parts = Vec::new();
            for block in layout.iter().copied().filter(|b| look.place(*b) == place) {
                let part = match block {
                    ScheduledBlock::Picture => self.look_picture(Art::Scheduled, look.picture, look.picture_file.as_deref(), look.picture_size, cx),
                    ScheduledBlock::About => about.take().map(|a| div().max_w(px(620.)).child(a).into_any_element()),
                    ScheduledBlock::Numbers => numbers(stats, &rules::SCHEDULED_NUMBERS, &look.hidden_numbers, on_picture, cx),
                };
                parts.extend(part);
            }
            zones.push((place, parts));
        }
        self.render_customized_page(Slot::Scheduled, backdrop, zones, true, 1000., window, cx)
    }

    /// The page's picture at the size chosen: the bear (which can be poked)
    /// or the user's own. None for no picture, or one whose file is gone.
    fn look_picture(&mut self, art: Art, picture: Picture, file: Option<&str>, size: rules::PictureSize, cx: &mut Context<Self>) -> Option<AnyElement> {
        let side = size.side();
        let el = match picture {
            Picture::None => return None,
            Picture::Bear => self.render_page_picture(art, None, art.fit(side, side), cx),
            Picture::Own => {
                let path = looks::path(file)?;
                let (w, h) = looks::size(file)?;
                let scale = (f64::from(side) / w.max(h)).min(f64::from(side) / w.max(1.)).min(f64::from(side) / h.max(1.));
                self.render_page_picture(art, Some(path), ((w * scale) as f32, (h * scale) as f32), cx)
            }
        };
        // Room under the bear for what it says when it's poked.
        Some(div().when(picture == Picture::Bear, |s| s.pb(px(4.))).child(el).into_any_element())
    }

    /// Today's quote (`LibrarianQuote`): the words, who said them and what
    /// they do, where it's from in the tooltip, and a button for another for
    /// the rest of the day. Right-click copies it.
    fn render_quote(&mut self, look: &rules::LibrarianLook, on_picture: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let day = rules::quote_day(crate::now_ms(), &Local);
        let quote = rules::pick_quote(&day, look.quote_skips_on(&day))?;
        let secondary = style::secondary_text(cx);
        let copy = format!("\u{201C}{}\u{201D}\n{}", quote.text, quote.author);
        let words = div()
            .id("look-quote-words")
            .flex_1()
            .min_w_0()
            .tip(quote.source)
            .child(v_flex().gap(px(6.)).child(div().text_size(px(17.)).line_height(px(23.)).font_family("Georgia").font_weight(FontWeight::MEDIUM).child(format!("\u{201C}{}\u{201D}", quote.text))).child(div().text_size(px(style::size::LABEL)).text_color(secondary).child(format!("{}, {}", quote.author, quote.role.to_lowercase()))))
            .on_mouse_down(MouseButton::Right, move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy.clone())));
        let again = crate::highlights::tipped(div().id("look-quote-again").flex_none().size(px(22.)).rounded(px(5.)).flex().items_center().justify_center().cursor_pointer().text_color(secondary), "Show another quote for today")
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                let mut look = this.librarian_look();
                look.skip_quote(&day);
                this.set_librarian_look(look, cx);
            }))
            .child(crate::icons::symbol("arrow.clockwise").with_size(px(11.)));
        Some(h_flex().items_start().gap(px(12.)).px(px(16.)).py(px(14.)).rounded(px(12.)).bg(tile_fill(on_picture, cx)).child(words).child(again).into_any_element())
    }

    /// Threadbear activity: a year of days shaded by how many threads you
    /// started each day, from the threads the app already has. Threads in a
    /// private folder aren't counted.
    fn render_thread_activity(&mut self, on_picture: bool, cx: &mut Context<Self>) -> AnyElement {
        let this_year = Local::now().year();
        let year = self.looks.activity_year.unwrap_or(this_year);
        let home = std::env::var("HOME").unwrap_or_default();
        let private = self.runner_settings.as_ref().map(threadbear_core::runner_settings::RunnerSettings::decode).map(|s| s.private_folders).unwrap_or_default();
        let threads: Vec<&threadbear_core::inbox::ThreadSummary> = self.threads.values().map(|t| &**t).collect();
        let first = rules::first_thread_year(threads.iter().copied(), &Local).unwrap_or(this_year).min(this_year);
        let activity = rules::thread_activity(year, threads.iter().copied(), &private, &home, &Local);
        let subtitle = format!("{} {} started in {year}", activity.total, if activity.total == 1 { "thread" } else { "threads" });
        let readout = match self.looks.hovered.as_ref().filter(|(card, _)| *card == "activity") {
            Some((_, day)) => match day.count {
                0 => format!("No threads on {}", rules::day_title(&day.date)),
                1 => format!("1 thread on {}", rules::day_title(&day.date)),
                n => format!("{n} threads on {}", rules::day_title(&day.date)),
            },
            None => "Threads in a private folder aren't counted".into(),
        };
        let stepper = self.year_stepper("activity", year, first, this_year, None, cx);
        let grid = self.contribution_grid("activity", &activity, None, cx);
        calendar_card("Threadbear activity", subtitle, stepper, grid, readout, on_picture, cx)
    }

    /// Someone's public GitHub contributions for a year, as on their GitHub
    /// profile. Hovering a day says its count; clicking one opens that day
    /// on their profile.
    fn render_github_activity(&mut self, user: &str, on_picture: bool, cx: &mut Context<Self>) -> AnyElement {
        let name = rules::github_user(user);
        let this_year = Local::now().year();
        let year = self.looks.github_year.unwrap_or(this_year);
        self.load_github_year(&name, year, false, cx);
        let k = key(&name, year);
        let state = self.looks.github.states.get(&k).cloned();
        let fetching = self.looks.github.fetching.contains(&k);
        let subtitle = match &state {
            Some(Fetched::Loaded(c)) => format!("{} {} in {year}", c.total, if c.total == 1 { "contribution" } else { "contributions" }),
            Some(Fetched::Failed(m)) => m.clone(),
            _ => format!("Loading @{name}'s {year}…"),
        };
        let readout = match self.looks.hovered.as_ref().filter(|(card, _)| *card == "github") {
            Some((_, day)) => match day.count {
                0 => format!("No contributions on {}", rules::day_title(&day.date)),
                1 => format!("1 contribution on {}", rules::day_title(&day.date)),
                n => format!("{n} contributions on {}", rules::day_title(&day.date)),
            },
            None if self.looks.github.stale.contains(&k) => "GitHub isn't answering right now, so this is the last copy".into(),
            None => format!("@{name}'s public contributions"),
        };
        let refresh_name = name.clone();
        let refresh = crate::highlights::tipped(div().id("look-github-refresh").flex_none().size(px(22.)).rounded(px(5.)).flex().items_center().justify_center().cursor_pointer().when(fetching, |s| s.opacity(0.4)), "Fetch this year from GitHub again")
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.load_github_year(&refresh_name, year, true, cx)))
            .child(crate::icons::symbol("arrow.clockwise").with_size(px(11.)))
            .into_any_element();
        let stepper = self.year_stepper("github", year, 2008, this_year, Some(refresh), cx);
        let grid = match state {
            Some(Fetched::Loaded(c)) => self.contribution_grid("github", &c, Some(name.clone()), cx),
            Some(Fetched::Failed(_)) => {
                let again = name.clone();
                div().h(px(CELL * 7. + 18.)).flex().items_center().child(
                    div().id("look-github-again").test_support().a11y_role(Role::GenericContainer).aria_label("Fetch this year from GitHub again").child(Button::new("look-github-again-button").label("Try again").small().secondary().tooltip("Fetch this year from GitHub again").on_click(cx.listener(move |this, _, _, cx| this.load_github_year(&again, year, true, cx)))),
                ).into_any_element()
            }
            _ => div().h(px(CELL * 7. + 18.)).flex().items_center().justify_center().child(Spinner::new().with_size(px(14.))).into_any_element(),
        };
        calendar_card("GitHub activity", subtitle, stepper, grid, readout, on_picture, cx)
    }

    /// The year with arrows either side, and anything after them.
    fn year_stepper(&mut self, card: &'static str, year: i32, first: i32, last: i32, extra: Option<AnyElement>, cx: &mut Context<Self>) -> AnyElement {
        let arrow = |id: &'static str, symbol: &'static str, tip: &'static str, enabled: bool, to: i32, cx: &mut Context<Self>| {
            crate::highlights::tipped(div().id(SharedString::from(format!("look-{card}-{id}"))).flex_none().size(px(22.)).rounded(px(5.)).flex().items_center().justify_center().when(enabled, |s| s.cursor_pointer()).when(!enabled, |s| s.opacity(0.3)), tip)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if !enabled {
                        return;
                    }
                    match card {
                        "github" => this.looks.github_year = Some(to),
                        _ => this.looks.activity_year = Some(to),
                    }
                    this.looks.hovered = None;
                    cx.notify();
                }))
                .child(crate::icons::symbol(symbol).with_size(px(11.)))
        };
        h_flex()
            .flex_none()
            .gap(px(6.))
            .items_center()
            .child(arrow("before", "chevron.left", "The year before", year > first, year - 1, cx))
            .child(div().min_w(px(44.)).text_center().text_size(px(15.)).font_weight(FontWeight::MEDIUM).child(year.to_string()))
            .child(arrow("after", "chevron.right", "The year after", year < last, year + 1, cx))
            .children(extra)
            .into_any_element()
    }

    /// `ContributionGrid`: a square a day, a column a week, with month names
    /// above and Mon, Wed, Fri beside, as GitHub has them. With `user`, a
    /// day opens on their GitHub profile.
    fn contribution_grid(&mut self, card: &'static str, c: &Contributions, user: Option<String>, cx: &mut Context<Self>) -> AnyElement {
        let weeks = c.weeks();
        let secondary = style::secondary_text(cx);
        let hovered = self.looks.hovered.as_ref().filter(|(k, _)| *k == card).map(|(_, d)| d.date.clone());
        let ring = style::primary_opacity(cx, 0.9);
        let side = CELL - 2.;
        let mut months = div().relative().h(px(18.)).w(px(LABELS + weeks.len() as f32 * CELL));
        for (name, week) in c.month_starts() {
            months = months.child(div().absolute().top_0().left(px(LABELS + week as f32 * CELL)).text_size(px(10.)).text_color(secondary).child(name));
        }
        let mut days = v_flex().w(px(LABELS)).flex_none();
        for row in 0..7 {
            let label = match row {
                1 => "Mon",
                3 => "Wed",
                5 => "Fri",
                _ => "",
            };
            days = days.child(div().h(px(CELL)).flex().items_center().text_size(px(10.)).text_color(secondary).child(label));
        }
        let mut columns = h_flex().items_start();
        for (i, week) in weeks.iter().enumerate() {
            let mut column = v_flex();
            for (row, day) in week.iter().enumerate() {
                let cell = match day {
                    None => div().size(px(CELL)).into_any_element(),
                    Some(day) => {
                        let shade = shade(day.level, cx);
                        let ring_on = hovered.as_deref() == Some(day.date.as_str());
                        let (hover_day, open_day, user) = (day.clone(), day.date.clone(), user.clone());
                        div()
                            .id(SharedString::from(format!("look-{card}-day-{i}-{row}")))
                            .size(px(CELL))
                            .p(px(1.))
                            .when(user.is_some(), |s| s.cursor_pointer())
                            .on_hover(cx.listener(move |this, on: &bool, _, cx| {
                                if *on {
                                    this.looks.hovered = Some((card, hover_day.clone()));
                                } else if this.looks.hovered.as_ref().is_some_and(|(k, d)| *k == card && d.date == hover_day.date) {
                                    this.looks.hovered = None;
                                }
                                cx.notify();
                            }))
                            .on_click(move |_, _, cx| {
                                if let Some(user) = &user {
                                    crate::open_gate::open_web(&format!("https://github.com/{user}?tab=overview&from={open_day}&to={open_day}"), cx);
                                }
                            })
                            .child(div().size(px(side)).rounded(px(2.)).bg(shade).when(ring_on, |s| s.border_1().border_color(ring)))
                            .into_any_element()
                    }
                };
                column = column.child(cell);
            }
            columns = columns.child(column);
        }
        let label = format!("{} in {}", c.total, c.year);
        div()
            .id(SharedString::from(format!("look-{card}-grid")))
            .test_support()
            .a11y_role(Role::Image)
            .aria_label(label)
            .w_full()
            .overflow_x_scroll()
            .child(v_flex().child(months).child(h_flex().items_start().child(days).child(columns)))
            .into_any_element()
    }
}

/// A day's square, at 100%, and the room for the day names beside them.
const CELL: f32 = 11.;
const LABELS: f32 = 30.;

/// Grey from faint to full, the page's own ink in light and dark.
fn shade(level: u8, cx: &App) -> Hsla {
    const OPACITY: [f32; 5] = [0.07, 0.22, 0.42, 0.66, 0.92];
    style::primary_opacity(cx, OPACITY[usize::from(level.min(4))])
}

/// A calendar card's frame: the title and what it counts, the years, the
/// calendar, then the readout and Less to More.
fn calendar_card(title: &'static str, subtitle: String, stepper: AnyElement, grid: AnyElement, readout: String, on_picture: bool, cx: &App) -> AnyElement {
    let secondary = style::secondary_text(cx);
    let mut legend = h_flex().flex_none().gap(px(4.)).items_center().text_size(px(11.)).text_color(secondary).child("Less");
    for level in 0..5 {
        legend = legend.child(div().size(px(10.)).rounded(px(2.)).bg(shade(level, cx)));
    }
    legend = legend.child("More");
    v_flex()
        .w_full()
        .gap(px(12.))
        .p(px(16.))
        .rounded(px(12.))
        .bg(tile_fill(on_picture, cx))
        .child(h_flex().w_full().items_start().gap(px(12.)).child(v_flex().flex_1().min_w_0().gap(px(3.)).child(div().text_size(px(15.)).font_semibold().child(title)).child(div().text_size(px(style::size::LABEL)).text_color(secondary).child(subtitle))).child(stepper))
        .child(grid)
        .child(h_flex().w_full().gap(px(12.)).items_center().child(div().flex_1().min_w_0().truncate().text_size(px(style::size::LABEL)).text_color(secondary).child(readout)).child(legend))
        .into_any_element()
}

/// A customized page's numbers: the tiles left after the hidden ones, or
/// nothing when they're all hidden. `names` are the keys, in tile order.
fn numbers(stats: &[HomeStat], names: &[(&str, &str)], hidden: &std::collections::BTreeSet<String>, on_picture: bool, cx: &App) -> Option<AnyElement> {
    let shown: Vec<(usize, &HomeStat)> = stats.iter().enumerate().filter(|(i, _)| names.get(*i).is_none_or(|(k, _)| !hidden.contains(*k))).collect();
    (!shown.is_empty()).then(|| h_flex().flex_wrap().max_w(px(620.)).gap(px(14.)).children(shown.into_iter().map(|(i, stat)| home_stat_tile_on(i, stat, on_picture, cx))).into_any_element())
}

/// "Good morning, Wayne" by this Mac's clock, and the date and time under it.
fn greeting(name: &str, clock: bool, cx: &App) -> AnyElement {
    let now = Local::now();
    v_flex()
        .gap(px(6.))
        .child(div().text_size(px(28.)).line_height(px(34.)).font_semibold().child(rules::greeting(now.hour(), name)))
        .when(clock, |s| s.child(div().text_size(px(style::size::LABEL)).text_color(style::secondary_text(cx)).child(now.format("%A %-d %B, %-I:%M %p").to_string())))
        .into_any_element()
}
