// The Librarian page laid out as you set it (a port of the Swift app's
// CustomizedLibrarianPage and LibrarianLookEditor): with Settings > General >
// Look's Customize the Librarian page on, a greeting by the clock, your
// picture or the bear or none, a background photo, the title and status (each
// part can go), a quote of the day, the numbers you want, a year of your
// threads and of your GitHub contributions, and the settings, each at the top,
// middle or bottom or hidden. Customize at the top right opens the panel
// beside the page, which changes it as you go, and holds the profile page's
// section (profile_page.rs), made from this page's look.
use crate::a11y::A11yRole as _;
use crate::app::Threadbear;
use crate::home_page::{home_stat_tile_on, home_status_line, home_title_parts, Art};
use crate::looks::{
    self, labeled, layout_row, note, panel_section, switch_row, tile_fill, Page, Slot,
};
use crate::style::{self, px};
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, Sizable as _, StyledExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use threadbear_core::chrono::{Local, Timelike as _};
use threadbear_core::librarian::{self as rules, State};
use threadbear_core::looks::{
    github_user_clean, quotes, AboutPart, Greeting, LibrarianBlock, LibrarianLook, Picture,
    PictureLook as _, Place,
};

/// The numbers' keys, in the order core's `librarian::stats` makes them,
/// with their names for the panel's switches.
pub(crate) const NUMBERS: [(&str, &str); 6] = [
    ("looks", "Looks today"),
    ("skipped", "Skipped today"),
    ("tokens", "Tokens today"),
    ("made", "Projects it made"),
    ("filed", "Threads it filed"),
    ("waiting", "Waiting"),
];

/// What the page and its panel hold.
#[derive(Default)]
pub(crate) struct LibrarianLookState {
    /// The panel's Name and GitHub user name boxes, made when it first opens.
    greeting_name: Option<(Entity<InputState>, Subscription)>,
    github_name: Option<(Entity<InputState>, Subscription)>,
    /// Set by Reset, so the boxes empty on the next draw (they need the window).
    refill: bool,
    /// Redraws on the minute while the greeting or the quote shows, so the
    /// morning turns to afternoon and the quote to the next day's.
    tick: Option<Task<()>>,
}

const CUSTOMIZE_TIP: &str =
    "Change the picture, the background, and where each part of this page sits";

impl Threadbear {
    /// Called when the look changes, from wherever: the profile page made
    /// from it goes again (profile_page.rs).
    pub(crate) fn librarian_look_changed(&mut self, cx: &mut Context<Self>) {
        self.profile_look_changed(cx);
    }

    /// The page with Customize on.
    pub(crate) fn render_customized_librarian(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let look: LibrarianLook = self.look();
        self.keep_clock(
            look.shows(LibrarianBlock::Greeting) || look.shows(LibrarianBlock::Quote),
            cx,
        );
        let pic = look.pic().clone();
        let backdrop = pic.background.as_deref().and_then(|bg| {
            self.render_backdrop(
                Slot::Page(Page::Librarian),
                bg,
                pic.background_dim,
                pic.background_focus,
                cx,
            )
        });
        let on_picture = backdrop.is_some();
        let mut zones: [Vec<AnyElement>; 3] = Default::default();
        for block in look.layout() {
            let zone = match look.place(block) {
                Place::Top => 0,
                Place::Middle => 1,
                Place::Bottom => 2,
            };
            if let Some(part) = self.librarian_part(block, &look, on_picture, cx) {
                zones[zone].push(part);
            }
        }
        let mut buttons = vec![];
        if let Some(url) = self.published_profile_url().filter(|_| look.profile_button) {
            buttons.push(looks::kit_button("view-profile-page", "View profile page", Some("person.crop.circle"), "Open your public profile page in the browser. Customize > Profile page turns this button off", false, false, move |_, _, cx| {
                crate::open_gate::open_web(&url, cx);
            }));
        }
        let editor = (self.looks.editing == Some(Page::Librarian))
            .then(|| self.render_librarian_editor(window, cx));
        self.render_customized_page(
            Page::Librarian,
            backdrop,
            zones,
            false,
            1000.,
            CUSTOMIZE_TIP,
            buttons,
            editor,
            cx,
        )
    }

    /// A minute's tick while something on the page reads the clock.
    fn keep_clock(&mut self, wanted: bool, cx: &mut Context<Self>) {
        if !wanted {
            self.librarian_look.tick = None;
            return;
        }
        if self.librarian_look.tick.is_some() {
            return;
        }
        self.librarian_look.tick = Some(cx.spawn(async move |this, cx| loop {
            let wait = 60 - u64::from(Local::now().second());
            cx.background_executor()
                .timer(std::time::Duration::from_secs(wait.max(1)))
                .await;
            if this.update(cx, |_, cx| cx.notify()).is_err() {
                break;
            }
        }));
    }

    fn librarian_part(
        &mut self,
        block: LibrarianBlock,
        look: &LibrarianLook,
        on_picture: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let narrow = |el: AnyElement| div().w_full().max_w(px(620.)).child(el).into_any_element();
        let wide = |el: AnyElement| div().w_full().max_w(px(760.)).child(el).into_any_element();
        Some(match block {
            LibrarianBlock::Greeting => {
                narrow(greeting(&look.greeting_name, look.greeting_clock, cx))
            }
            LibrarianBlock::Quote => narrow(self.render_quote(look, on_picture, cx)?),
            LibrarianBlock::Picture => self.render_look_picture(
                look.pic().picture,
                look.pic().picture_file.as_deref(),
                look.pic().picture_size.side(),
                Some(Art::Librarian),
                cx,
            )?,
            LibrarianBlock::About => {
                let status = self.librarian.status.clone();
                let mut about = v_flex().gap(px(12.));
                if look.shows_part(AboutPart::Title) || look.shows_part(AboutPart::Line) {
                    about = about.child(home_title_parts(
                        look.shows_part(AboutPart::Title).then_some("Librarian"),
                        look.shows_part(AboutPart::Line)
                            .then_some("Keeps your projects tidy as your threads go."),
                        crate::librarian::MORE,
                        cx,
                    ));
                }
                if let Some(status) = status.filter(|_| look.shows_part(AboutPart::Status)) {
                    about = about.child(home_status_line(
                        &rules::state_text(&status),
                        status.state == State::Working,
                        cx,
                    ));
                }
                narrow(about.into_any_element())
            }
            LibrarianBlock::Numbers => {
                let status = self.librarian.status.clone()?;
                let threads: Vec<&threadbear_core::inbox::ThreadSummary> =
                    self.threads.values().map(|t| &**t).collect();
                let stats = rules::stats(&status, self.librarian.projects_made, &threads);
                let shown: Vec<_> = stats
                    .iter()
                    .zip(NUMBERS)
                    .filter(|(_, (k, _))| !look.hidden_numbers.contains(*k))
                    .map(|(s, _)| s.clone())
                    .collect();
                if shown.is_empty() {
                    return None;
                }
                h_flex()
                    .flex_wrap()
                    .max_w(px(620.))
                    .gap(px(14.))
                    .children(
                        shown
                            .iter()
                            .enumerate()
                            .map(|(i, s)| home_stat_tile_on(i, s, on_picture, cx)),
                    )
                    .into_any_element()
            }
            LibrarianBlock::Activity => wide(self.render_activity_card(on_picture, cx)),
            LibrarianBlock::Github => {
                wide(self.render_github_card(&look.github_user, on_picture, cx))
            }
            LibrarianBlock::Settings => {
                let status = self.librarian.status.clone()?;
                narrow(self.render_librarian_settings(&status, cx))
            }
        })
    }

    /// A page's picture: the bear (which says things when poked), your own,
    /// or nothing when it's gone.
    pub(crate) fn render_look_picture(
        &mut self,
        picture: Picture,
        file: Option<&str>,
        side: f32,
        art: Option<Art>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        match (picture, art) {
            (Picture::None, _) => None,
            (Picture::Bear, Some(art)) => Some(
                div()
                    .pb(px(4.))
                    .child(self.render_poke_picture(art, Some(side), cx))
                    .into_any_element(),
            ),
            (Picture::Bear, None) => None,
            (Picture::Own, _) => {
                let file = file?;
                let (w, h) = looks::dims(file)?;
                let scale = side / w.max(h);
                let path = looks::path(file)?;
                Some(
                    div()
                        .id("look-picture")
                        .test_support()
                        .role(Role::Image)
                        .aria_label("Your picture")
                        .w(px(w * scale))
                        .h(px(h * scale))
                        .child(img(path).size_full().object_fit(ObjectFit::Contain))
                        .into_any_element(),
                )
            }
        }
    }

    /// Today's quote (`LibrarianQuote`): the words, who said them and what
    /// they do, with where it's from in the tooltip. The refresh button shows
    /// another for the rest of the day; the day turns at 4 am on this Mac's
    /// clock. Right-click to copy it.
    fn render_quote(
        &mut self,
        look: &LibrarianLook,
        on_picture: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let day = quotes::day_key(Local::now().naive_local());
        let quote = quotes::pick(&day, look.quote_skips_on(&day))?;
        let secondary = style::secondary_text(cx);
        let copied = format!("\u{201C}{}\u{201D}\n{}", quote.text, quote.author);
        let refresh = crate::highlights::tipped(
            div()
                .id("quote-another")
                .flex_none()
                .size(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .cursor_pointer()
                .hover(|s| s.bg(gpui_kit::hsla(0., 0., 0.5, 0.12))),
            "Show another quote for today",
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            let day = day.clone();
            this.edit_look::<LibrarianLook>(cx, |l| l.skip_quote(&day));
        }))
        .child(
            crate::icons::symbol("arrow.clockwise")
                .with_size(px(11.))
                .text_color(secondary),
        );
        Some(
            h_flex()
                .id("quote-card")
                .w_full()
                .items_start()
                .gap(px(12.))
                .px(px(16.))
                .py(px(14.))
                .rounded(px(12.))
                .bg(tile_fill(on_picture, cx))
                .child(
                    v_flex()
                        .id("quote-words")
                        .test_support()
                        .a11y_role(Role::GenericContainer)
                        .aria_label(quote.text)
                        .flex_1()
                        .min_w_0()
                        .gap(px(6.))
                        .tip(quote.source)
                        .child(
                            div()
                                .font_family("Georgia")
                                .text_size(px(17.))
                                .line_height(px(23.))
                                .font_medium()
                                .child(format!("\u{201C}{}\u{201D}", quote.text)),
                        )
                        .child(
                            div()
                                .text_size(px(style::size::LABEL))
                                .line_height(px(16.))
                                .text_color(secondary)
                                .child(format!("{}, {}", quote.author, quote.role.to_lowercase())),
                        ),
                )
                .child(refresh)
                .context_menu(move |menu, _, _| {
                    let copied = copied.clone();
                    menu.item(
                        PopupMenuItem::new("Copy Quote")
                            .icon(crate::icons::symbol("doc.on.doc"))
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()))
                            }),
                    )
                })
                .into_any_element(),
        )
    }

    // The panel.

    fn render_librarian_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let look: LibrarianLook = self.look();
        let (greeting_box, github_box) = self.librarian_boxes(&look, window, cx);
        let mut sections = vec![];

        // Greeting.
        let mut rows = vec![switch_row(
            "look-greeting",
            "Greet me",
            look.greeting_shown,
            "Good morning, afternoon, evening or night at the top of the page, by this Mac's clock",
            false,
            cx,
            |this, on, cx| this.show_librarian_block(LibrarianBlock::Greeting, on, cx),
        )];
        if look.greeting_shown {
            rows.push(labeled(
                "Name",
                div()
                    .id("look-greeting-name")
                    .w(px(150.))
                    .tip("The name it greets you by. Leave it empty for just Good morning")
                    .child(Input::new(&greeting_box).small())
                    .into_any_element(),
                cx,
            ));
            rows.push(switch_row(
                "look-greeting-clock",
                "Show the date and time",
                look.greeting_clock,
                "Today's date and the time under the greeting",
                false,
                cx,
                |this, on, cx| this.edit_look::<LibrarianLook>(cx, |l| l.greeting_clock = on),
            ));
        }
        sections.push(panel_section("Greeting", Some(("look-greeting-info", "Good morning from 5, good afternoon from 12, good evening from 5 pm and good night from 10 pm, by this Mac's own clock and time zone. It changes on the minute. The name stays on this Mac.")), rows, cx));

        sections.push(self.render_picture_section::<LibrarianLook>("Show the librarian bear", cx));
        sections.push(self.render_background_section::<LibrarianLook>(cx));

        // Quote of the day.
        sections.push(panel_section(
            "Quote of the day",
            Some(("look-quote-info", "A quote from a writer, scientist, engineer, artist or athlete, the same all day and a new one at 4 am by this Mac's clock. The refresh button beside it shows another for the rest of the day, and right-click copies it. Hover it for where it's from. The quotes ship with Threadbear: nothing is fetched or sent, and none are from politicians, religious figures or tech founders. Move it with Layout.")),
            vec![switch_row("look-quote", "Show a quote a day", look.quote_shown, "A quote a day, changing at 4 am, with a button for another", false, cx, |this, on, cx| this.show_librarian_block(LibrarianBlock::Quote, on, cx))],
            cx,
        ));

        // Layout.
        let rows = LibrarianBlock::ALL
            .iter()
            .map(|&block| {
                let on = match block {
                    LibrarianBlock::Greeting => look.greeting_shown,
                    LibrarianBlock::Quote => look.quote_shown,
                    LibrarianBlock::Activity => look.activity_shown,
                    _ => !look.hidden.contains(&block),
                };
                let tip = match block {
                    LibrarianBlock::Github => "Show your GitHub activity. Needs a user name, below".to_owned(),
                    LibrarianBlock::Quote => "A quote a day, changing at 4 am, with a button for another".to_owned(),
                    LibrarianBlock::Activity => "Threadbear activity: a year of days shaded by how many threads you started each day, as on your profile page. Threads in a private folder aren't counted".to_owned(),
                    _ => format!("Show or hide {} on the page", block.title().to_lowercase()),
                };
                layout_row(format!("look-layout-{}", block.raw()), block.title(), on, tip, look.place(block), cx, move |this, on, cx| this.show_librarian_block(block, on, cx), move |this, place, cx| this.edit_look::<LibrarianLook>(cx, |l| l.set_place(place, block)))
            })
            .collect();
        sections.push(panel_section("Layout", None, rows, cx));

        // Title and status.
        let about_hidden = look.hidden.contains(&LibrarianBlock::About);
        let rows = AboutPart::ALL
            .iter()
            .map(|&part| {
                let tip = match part {
                    AboutPart::Title => "Show or hide the word Librarian at the top",
                    AboutPart::Line => "Show or hide the line under the title, Keeps your projects tidy as your threads go",
                    AboutPart::Status => "Show or hide what the librarian is doing now",
                };
                switch_row(format!("look-about-{}", part.raw()), part.title(), look.shows_part(part), tip, about_hidden, cx, move |this, on, cx| this.edit_look::<LibrarianLook>(cx, |l| l.set_part_shown(on, part)))
            })
            .collect();
        sections.push(panel_section("Title and status", None, rows, cx));

        // Numbers.
        let numbers_hidden = look.hidden.contains(&LibrarianBlock::Numbers);
        let rows = NUMBERS
            .iter()
            .map(|&(key, title)| {
                switch_row(
                    format!("look-number-{key}"),
                    title,
                    !look.hidden_numbers.contains(key),
                    format!("Show or hide {}", title.to_lowercase()),
                    numbers_hidden,
                    cx,
                    move |this, on, cx| {
                        this.edit_look::<LibrarianLook>(cx, |l| {
                            if on {
                                l.hidden_numbers.remove(key);
                            } else {
                                l.hidden_numbers.insert(key.to_owned());
                            }
                        })
                    },
                )
            })
            .collect();
        sections.push(panel_section("Numbers", None, rows, cx));

        // GitHub activity.
        let typed = github_box.read(cx).value().to_string();
        let mut rows = vec![labeled(
            "User name",
            div()
                .id("look-github-name")
                .w(px(150.))
                .tip("Your GitHub user name, or a link to your profile")
                .child(Input::new(&github_box).small())
                .into_any_element(),
            cx,
        )];
        if !typed.trim().is_empty() && github_user_clean(&typed).is_empty() {
            rows.push(note("That isn't a GitHub user name.", cx));
        }
        sections.push(panel_section("GitHub activity", Some(("look-github-info", "Your public contributions for each year, as on your GitHub profile. Fetched from the GitHub Contributions API (github-contributions-api.jogruber.de), which reads that calendar; only the user name is sent, and no token. Private work counts only if your profile shows private contributions.")), rows, cx));

        sections.push(panel_section(
            "",
            None,
            vec![looks::kit_button("look-reset", "Reset to the standard page", None, "Put the bear, the layout and every part back as Threadbear ships them, and take away your pictures", look.is_standard(), false, cx.listener(|this, _, _, cx| {
                this.save_look(LibrarianLook::default(), cx);
                this.librarian_look.refill = true;
                cx.notify();
            }))],
            cx,
        ));
        sections.extend(self.render_profile_sections(&look, window, cx));
        self.render_look_editor(Page::Librarian, sections, cx)
    }

    /// A part's switch. The first time the greeting goes on, it suggests
    /// the Mac account's first name, which can be changed or cleared.
    fn show_librarian_block(&mut self, block: LibrarianBlock, on: bool, cx: &mut Context<Self>) {
        let look: LibrarianLook = self.look();
        let suggest = block == LibrarianBlock::Greeting
            && on
            && !look.greeting_shown
            && look.greeting_name.is_empty();
        let name = if suggest {
            Greeting::first_name(&full_user_name())
        } else {
            String::new()
        };
        self.edit_look::<LibrarianLook>(cx, |l| {
            if suggest {
                l.greeting_name = name.clone();
            }
            l.set_shown(on, block);
        });
        if suggest && !name.is_empty() {
            self.librarian_look.refill = true;
        }
    }

    /// The Name and user name boxes, made once, and filled again after Reset.
    #[expect(
        clippy::expect_used,
        reason = "only drawn while what it unwraps is set, just above or by its caller"
    )]
    fn librarian_boxes(
        &mut self,
        look: &LibrarianLook,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<InputState>, Entity<InputState>) {
        if self.librarian_look.greeting_name.is_none() {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("No name")
                    .default_value(look.greeting_name.clone())
            });
            let sub = cx.subscribe(&input, |this, input, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    let typed = input.read(cx).value().to_string();
                    this.edit_look::<LibrarianLook>(cx, |l| l.greeting_name = typed);
                }
            });
            self.librarian_look.greeting_name = Some((input, sub));
        }
        if self.librarian_look.github_name.is_none() {
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("octocat")
                    .default_value(look.github_user.clone())
            });
            let sub = cx.subscribe(&input, |this, input, ev: &InputEvent, cx| match ev {
                // Saved as it's typed once it's a real name, or cleared.
                InputEvent::Change => {
                    let typed = input.read(cx).value().to_string();
                    let clean = github_user_clean(&typed);
                    if typed.trim().is_empty() || !clean.is_empty() {
                        this.edit_look::<LibrarianLook>(cx, |l| l.github_user = clean);
                    }
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => this.librarian_look.refill = true,
                _ => {}
            });
            self.librarian_look.github_name = Some((input, sub));
        }
        let greeting = self
            .librarian_look
            .greeting_name
            .as_ref()
            .map(|(i, _)| i.clone())
            .expect("made above");
        let github = self
            .librarian_look
            .github_name
            .as_ref()
            .map(|(i, _)| i.clone())
            .expect("made above");
        if std::mem::take(&mut self.librarian_look.refill) {
            let (name, user) = (look.greeting_name.clone(), look.github_user.clone());
            greeting.update(cx, |s, cx| s.set_value(name, window, cx));
            github.update(cx, |s, cx| s.set_value(user, window, cx));
        }
        (greeting, github)
    }

    /// The page in the dump, with Customize on: its parts in order, and
    /// what the greeting and the quote say.
    #[cfg(feature = "automation")]
    pub(crate) fn customized_librarian_dump(&self) -> serde_json::Value {
        let look: LibrarianLook = self.look();
        let day = quotes::day_key(Local::now().naive_local());
        serde_json::json!({
            "parts": look.layout().iter().map(|b| b.raw()).collect::<Vec<_>>(),
            "greeting": look.shows(LibrarianBlock::Greeting).then(|| Greeting::text(Local::now().hour(), &look.greeting_name)),
            "quote": look.shows(LibrarianBlock::Quote).then(|| quotes::pick(&day, look.quote_skips_on(&day)).map(|q| q.text)).flatten(),
            "picture": look.pic().picture.raw(),
            "background": look.pic().background,
            "editing": self.looks.editing == Some(Page::Librarian),
        })
    }
}

/// "Good morning, Wayne", by this Mac's clock, with the date and time under it.
fn greeting(name: &str, clock: bool, cx: &App) -> AnyElement {
    let now = Local::now();
    v_flex()
        .gap(px(6.))
        .child(
            div()
                .id("look-greeting-text")
                .test_support()
                .a11y_role(Role::GenericContainer)
                .aria_label(Greeting::text(now.hour(), name))
                .text_size(px(28.))
                .line_height(px(34.))
                .font_semibold()
                .child(Greeting::text(now.hour(), name)),
        )
        .when(clock, |s| {
            s.child(
                div()
                    .text_size(px(style::size::LABEL))
                    .line_height(px(16.))
                    .text_color(style::secondary_text(cx))
                    .child(now.format("%A, %B %-d at %-I:%M %p").to_string()),
            )
        })
        .into_any_element()
}

/// The Mac account's full name ("Wayne Sutton"), for the greeting's
/// suggestion; empty where there's none to read.
pub(crate) fn full_user_name() -> String {
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn tb_full_user_name() -> *const std::ffi::c_char;
        }
        // SAFETY: takes nothing and returns a C string the shim owns, or null.
        let name = unsafe { tb_full_user_name() };
        if !name.is_null() {
            // SAFETY: name is non-null here and NUL-terminated.
            return unsafe { std::ffi::CStr::from_ptr(name) }
                .to_string_lossy()
                .into_owned();
        }
    }
    std::env::var("USER").unwrap_or_default()
}
