// The Librarian and Scheduled pages' Customize panels, a port of the Swift
// app's LibrarianLookEditor, ScheduledLookEditor and LookEditorSections:
// beside the page, so every change shows as it's made. The picture (the bear,
// one of your own, or none), a background photo, where it sits and how faded
// it is, where each part sits and whether it shows, which numbers show, and
// on the Librarian page the greeting, the quote and whose GitHub activity.
// Pictures can be dropped on their wells as well as chosen. Every change is
// saved as it's made, so Done only closes the panel.
use crate::a11y::A11yRole as _;
use crate::app::Threadbear;
use crate::home_page::info_tip;
use crate::looks::{self, Kind, Slot};
use crate::mac_switch::MacSwitch;
use crate::style;
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::slider::Slider;
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _, StyledExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
// The zoom-aware px (style.rs), in place of GPUI's.
use crate::style::px;
use std::rc::Rc;
use threadbear_core::looks::{self as rules, AboutPart, LibrarianBlock, Picture, PictureSize, Place, ScheduledBlock};

const GREETING_ABOUT: &str = "Good morning from 5, good afternoon from 12, good evening from 5 pm and good night from 10 pm, by this Mac's own clock and time zone. It changes on the minute. The name stays on this Mac.";
const QUOTE_ABOUT: &str = "A quote from a writer, scientist, engineer, artist or athlete, the same all day and a new one at 4 am by this Mac's clock. The refresh button beside it shows another for the rest of the day, and right-click copies it. Hover it for where it's from. The quotes ship with Threadbear: nothing is fetched or sent, and none are from politicians, religious figures or tech founders. Move it with Layout.";
const GITHUB_ABOUT: &str = "Your public contributions for each year, as on your GitHub profile. Fetched from the GitHub Contributions API (github-contributions-api.jogruber.de), which reads that calendar; only the user name is sent, and no token. Private work counts only if your profile shows private contributions.";

impl Threadbear {
    /// The panel for a page, its header over its sections.
    pub(crate) fn render_look_editor(&mut self, slot: Slot, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let sections = match slot {
            Slot::Librarian => self.librarian_editor_sections(window, cx),
            Slot::Scheduled => self.scheduled_editor_sections(cx),
            Slot::Main => Vec::new(),
        };
        let header = h_flex()
            .flex_none()
            .px(px(16.))
            .py(px(12.))
            .border_b_1()
            .border_color(style::hairline(cx))
            .child(div().flex_1().text_size(px(15.)).font_semibold().child("Customize"))
            .child(
                div().id("look-done").test_support().a11y_role(Role::GenericContainer).aria_label("Close Customize. Changes are already saved").child(
                    Button::new("look-done-button").label("Done").small().secondary().tooltip("Close Customize. Changes are already saved").on_click(cx.listener(|this, _, _, cx| {
                        this.looks.editing = None;
                        cx.notify();
                    })),
                ),
            );
        v_flex()
            .size_full()
            .child(header)
            .child(div().id(SharedString::from(format!("look-editor-{slot:?}"))).flex_1().min_h_0().overflow_y_scroll().child(v_flex().gap(px(18.)).px(px(12.)).pt(px(14.)).pb(px(32.)).children(sections)))
            .into_any_element()
    }

    fn librarian_editor_sections(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let look = self.librarian_look();
        let mut out = Vec::new();

        // Greeting.
        let mut rows = vec![row("Greet me", self.look_switch("look-greeting", look.greeting_shown, "Good morning, afternoon, evening or night at the top of the page, by this Mac's clock", false, cx, |this, on, window, cx| this.show_librarian_block(LibrarianBlock::Greeting, on, window, cx)), cx)];
        if look.greeting_shown {
            let name = self.look_field(Field::Greeting, &look.greeting_name, "No name", window, cx);
            rows.push(row("Name", tipped_field("look-greeting-name", "The name it greets you by. Leave it empty for just Good morning", name), cx));
            rows.push(row("Show the date and time", self.look_switch("look-greeting-clock", look.greeting_clock, "Today's date and the time under the greeting", false, cx, |this, on, _, cx| {
                let mut look = this.librarian_look();
                look.greeting_clock = on;
                this.set_librarian_look(look, cx);
            }), cx));
        }
        out.push(section("Greeting", Some(GREETING_ABOUT), rows, cx));

        out.push(self.picture_section(Slot::Librarian, "Show the librarian bear", cx));
        out.push(self.background_section(Slot::Librarian, cx));

        // Quote of the day.
        out.push(section("Quote of the day", Some(QUOTE_ABOUT), vec![row("Show a quote a day", self.look_switch("look-quote", look.quote_shown, "A quote a day, changing at 4 am, with a button for another", false, cx, |this, on, window, cx| this.show_librarian_block(LibrarianBlock::Quote, on, window, cx)), cx)], cx));

        // Layout.
        let rows = LibrarianBlock::ALL
            .into_iter()
            .map(|block| {
                let on = look.switched_on(block);
                let tip = match block {
                    LibrarianBlock::Github => "Show your GitHub activity. Needs a user name, below".to_owned(),
                    LibrarianBlock::Quote => "A quote a day, changing at 4 am, with a button for another".to_owned(),
                    LibrarianBlock::Activity => "Threadbear activity: a year of days shaded by how many threads you started each day. Threads in a private folder aren't counted".to_owned(),
                    _ => format!("Show or hide {} on the page", block.title().to_lowercase()),
                };
                let switch = self.look_switch_owned(format!("look-layout-{}", block.raw()), on, tip, false, cx, move |this, on, window, cx| this.show_librarian_block(block, on, window, cx));
                let place = self.place_menu(format!("look-place-{}", block.raw()), look.place(block), !on, cx, move |this, place, cx| {
                    let mut look = this.librarian_look();
                    look.set_place(place, block);
                    this.set_librarian_look(look, cx);
                });
                layout_row(block.title(), switch, place, cx)
            })
            .collect();
        out.push(section("Layout", None, rows, cx));

        // Title and status.
        let about_hidden = look.hidden.contains(&LibrarianBlock::About);
        let rows = AboutPart::ALL
            .into_iter()
            .map(|part| {
                let tip = match part {
                    AboutPart::Title => "Show or hide the word Librarian at the top",
                    AboutPart::Line => "Show or hide the line under the title, Keeps your projects tidy as your threads go",
                    AboutPart::Status => "Show or hide what the librarian is doing now",
                };
                row(part.title(), self.look_switch_owned(format!("look-about-{}", part.raw()), look.shows_part(part), tip.into(), about_hidden, cx, move |this, on, _, cx| {
                    let mut look = this.librarian_look();
                    look.set_part_shown(on, part);
                    this.set_librarian_look(look, cx);
                }), cx)
            })
            .collect();
        out.push(section("Title and status", None, rows, cx));

        out.push(self.numbers_section(Slot::Librarian, &rules::LIBRARIAN_NUMBERS, &look.hidden_numbers, look.hidden.contains(&LibrarianBlock::Numbers), cx));

        // GitHub activity.
        let field = self.look_field(Field::GitHub, &look.github_user, "octocat", window, cx);
        let mut rows = vec![row("User name", tipped_field("look-github-name", "Your GitHub user name, or a link to your profile", field), cx)];
        let typed = self.looks.github_typed.clone();
        if !typed.trim().is_empty() && rules::github_user(&typed).is_empty() {
            rows.push(note("That isn't a GitHub user name.", false, cx));
        }
        out.push(section("GitHub activity", Some(GITHUB_ABOUT), rows, cx));

        out.push(self.reset_button(Slot::Librarian, look.is_standard(), cx));
        out
    }

    fn scheduled_editor_sections(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let look = self.scheduled_look();
        let mut out = vec![self.picture_section(Slot::Scheduled, "Show the bear winding its alarm clock", cx), self.background_section(Slot::Scheduled, cx)];
        let rows = ScheduledBlock::ALL
            .into_iter()
            .map(|block| {
                let on = !look.hidden.contains(&block);
                let switch = self.look_switch_owned(format!("look-layout-{}", block.raw()), on, format!("Show or hide {} on the page", block.title().to_lowercase()), false, cx, move |this, on, _, cx| {
                    let mut look = this.scheduled_look();
                    look.set_shown(on, block);
                    this.set_scheduled_look(look, cx);
                });
                let place = self.place_menu(format!("look-place-{}", block.raw()), look.place(block), !on, cx, move |this, place, cx| {
                    let mut look = this.scheduled_look();
                    look.set_place(place, block);
                    this.set_scheduled_look(look, cx);
                });
                layout_row(block.title(), switch, place, cx)
            })
            .collect();
        out.push(section("Layout", None, rows, cx));
        out.push(self.numbers_section(Slot::Scheduled, &rules::SCHEDULED_NUMBERS, &look.hidden_numbers, look.hidden.contains(&ScheduledBlock::Numbers), cx));
        out.push(self.reset_button(Slot::Scheduled, look.is_standard(), cx));
        out
    }

    /// A Layout switch or its own section's: the first time the greeting goes
    /// on, it suggests the Mac account's first name, which can be changed.
    fn show_librarian_block(&mut self, block: LibrarianBlock, on: bool, window: &mut Window, cx: &mut Context<Self>) {
        let mut look = self.librarian_look();
        if block == LibrarianBlock::Greeting && on && !look.greeting_shown && look.greeting_name.is_empty() {
            look.greeting_name = rules::first_name(&full_user_name());
            if let Some(field) = &self.looks.greeting_name {
                let name = look.greeting_name.clone();
                field.update(cx, |f, cx| f.set_value(name, window, cx));
            }
        }
        look.set_shown(on, block);
        self.set_librarian_look(look, cx);
    }

    /// `LookPictureSection`: the bear, a picture of your own or none, and its size.
    fn picture_section(&mut self, slot: Slot, bear_tip: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let Some(look) = self.picture_look(slot) else { return div().into_any_element() };
        let picture = look.picture();
        let mut rows = Vec::new();
        let mut choice = h_flex().w_full().gap(px(2.)).p(px(2.)).rounded(px(8.)).bg(style::primary_opacity(cx, 0.08));
        for option in Picture::ALL {
            let on = option == picture;
            let tip = match option {
                Picture::Bear => bear_tip,
                Picture::Own => "Show a picture of your own",
                Picture::None => "No picture on the page",
            };
            let accent = style::control_accent(cx);
            choice = choice.child(
                crate::highlights::tipped(div().id(SharedString::from(format!("look-picture-{}-{slot:?}", option.raw()))).flex_1().py(px(4.)).rounded(px(6.)).text_center().text_size(px(style::size::LABEL)).cursor_pointer().when(on, |s| s.bg(accent).text_color(gpui_kit::white()).font_weight(FontWeight::MEDIUM)), tip)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if on {
                            return;
                        }
                        // Your picture with none chosen yet asks for one straight away.
                        let has_file = this.picture_look(slot).is_some_and(|l| l.picture_file().is_some());
                        if option == Picture::Own && !has_file {
                            this.choose_look_picture(slot, Kind::Picture, cx);
                            return;
                        }
                        this.edit_picture_look(slot, |l| l.set_picture(option), cx);
                    }))
                    .child(option.title()),
            );
        }
        rows.push(div().px(px(10.)).py(px(8.)).child(choice).into_any_element());
        if picture == Picture::Own {
            let me = cx.entity().downgrade();
            let dropped: Rc<dyn Fn(std::path::PathBuf, &mut Window, &mut App)> = Rc::new(move |path, _, cx| {
                me.update(cx, |this, cx| this.import_look_picture(slot, Kind::Picture, path, cx)).ok();
            });
            rows.push(div().px(px(10.)).py(px(8.)).child(looks::well(format!("look-picture-well-{slot:?}"), look.picture_file(), "Drop a picture here", false, dropped, cx)).into_any_element());
            let has = look.picture_file().is_some();
            let mut buttons = h_flex().gap(px(8.)).child(small_button(format!("look-picture-choose-{slot:?}"), if has { "Replace…" } else { "Choose…" }, "Pick a picture from your Mac. A copy is kept, so the original can move", cx, move |this, cx| this.choose_look_picture(slot, Kind::Picture, cx)));
            if has {
                buttons = buttons.child(small_button(format!("look-picture-remove-{slot:?}"), "Remove", "Go back to the bear and delete the copy", cx, move |this, cx| {
                    this.edit_picture_look(
                        slot,
                        |l| {
                            l.set_picture_file(None);
                            l.set_picture(Picture::Bear);
                        },
                        cx,
                    )
                }));
            }
            rows.push(div().px(px(10.)).py(px(6.)).child(buttons).into_any_element());
        }
        if picture != Picture::None {
            let size = look.picture_size();
            let me = cx.entity().downgrade();
            let menu = Button::new(SharedString::from(format!("look-picture-size-{slot:?}"))).label(size.title()).small().secondary().dropdown_caret(true).tooltip("How big the picture is on the page").dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for choice in PictureSize::ALL {
                    let me = me.clone();
                    menu = menu.item(PopupMenuItem::new(choice.title()).checked(choice == size).on_click(move |_, _, cx| {
                        me.update(cx, |this, cx| this.edit_picture_look(slot, |l| l.set_picture_size(choice), cx)).ok();
                    }));
                }
                menu
            });
            rows.push(row("Size", div().id(SharedString::from(format!("look-picture-size-holder-{slot:?}"))).test_support().a11y_role(Role::GenericContainer).aria_label("How big the picture is on the page").child(menu), cx));
        }
        if let Some((_, _, problem)) = self.looks.problem.as_ref().filter(|p| p.0 == slot && p.1 == Kind::Picture) {
            rows.push(note(problem, true, cx));
        }
        section("Picture", None, rows, cx)
    }

    /// `LookBackgroundSection`: a photo behind the page, where it sits when
    /// cropped, and how faded it is.
    fn background_section(&mut self, slot: Slot, cx: &mut Context<Self>) -> AnyElement {
        let Some(look) = self.picture_look(slot) else { return div().into_any_element() };
        let background = look.background().map(str::to_owned);
        let mut rows = Vec::new();
        let me = cx.entity().downgrade();
        let dropped: Rc<dyn Fn(std::path::PathBuf, &mut Window, &mut App)> = Rc::new(move |path, _, cx| {
            me.update(cx, |this, cx| this.import_look_picture(slot, Kind::Background, path, cx)).ok();
        });
        rows.push(div().px(px(10.)).py(px(8.)).child(looks::well(format!("look-background-well-{slot:?}"), background.as_deref(), "Drop a photo here", true, dropped, cx)).into_any_element());
        let mut buttons = h_flex().gap(px(8.)).child(small_button(format!("look-background-choose-{slot:?}"), if background.is_some() { "Replace…" } else { "Choose…" }, "Pick a photo or an animated GIF to fill the page behind everything", cx, move |this, cx| this.choose_look_picture(slot, Kind::Background, cx)));
        if background.is_some() {
            buttons = buttons.child(small_button(format!("look-background-remove-{slot:?}"), "Remove", "Take the background away and delete the copy", cx, move |this, cx| this.edit_picture_look(slot, |l| l.set_background(None), cx)));
        }
        rows.push(div().px(px(10.)).py(px(6.)).child(buttons).into_any_element());
        if let Some(name) = background.as_deref().filter(|n| looks::path(Some(n)).is_some()) {
            let me = cx.entity().downgrade();
            let set: Rc<dyn Fn(rules::BackdropFocus, &mut App)> = Rc::new(move |focus, cx| {
                me.update(cx, |this, cx| this.edit_picture_look(slot, |l| l.set_background_focus(focus), cx)).ok();
            });
            if let Some(position) = looks::positioner(slot, name, look.background_focus(), (200., 130.), set, cx) {
                rows.push(row("Position", position, cx));
            }
            let slider = self.fade_slider(slot, look.background_dim(), cx);
            rows.push(row("Fade", div().id(SharedString::from(format!("look-fade-{slot:?}"))).test_support().a11y_role(Role::GenericContainer).aria_label("How much the page's own colour covers the photo").w(px(150.)).tip("How much the page's own colour covers the photo, so text stays easy to read in light and dark").child(Slider::new(&slider).horizontal()), cx));
        }
        if let Some((_, _, problem)) = self.looks.problem.as_ref().filter(|p| p.0 == slot && p.1 == Kind::Background) {
            rows.push(note(problem, true, cx));
        }
        section("Background", None, rows, cx)
    }

    fn numbers_section(&mut self, slot: Slot, names: &'static [(&'static str, &'static str)], hidden: &std::collections::BTreeSet<String>, disabled: bool, cx: &mut Context<Self>) -> AnyElement {
        let rows = names
            .iter()
            .map(|(key, title)| {
                let key = *key;
                row(title, self.look_switch_owned(format!("look-number-{key}"), !hidden.contains(key), format!("Show or hide {}", title.to_lowercase()), disabled, cx, move |this, on, _, cx| {
                    let change = |set: &mut std::collections::BTreeSet<String>| {
                        if on {
                            set.remove(key);
                        } else {
                            set.insert(key.to_owned());
                        }
                    };
                    match slot {
                        Slot::Librarian => {
                            let mut look = this.librarian_look();
                            change(&mut look.hidden_numbers);
                            this.set_librarian_look(look, cx);
                        }
                        _ => {
                            let mut look = this.scheduled_look();
                            change(&mut look.hidden_numbers);
                            this.set_scheduled_look(look, cx);
                        }
                    }
                }), cx)
            })
            .collect();
        section("Numbers", None, rows, cx)
    }

    fn reset_button(&mut self, slot: Slot, standard: bool, cx: &mut Context<Self>) -> AnyElement {
        let tip = "Put the bear, the layout and every part back as Threadbear ships them, and take away your pictures";
        div()
            .px(px(10.))
            .child(
                div().id(SharedString::from(format!("look-reset-{slot:?}"))).test_support().a11y_role(Role::GenericContainer).aria_label(tip).child(
                    Button::new(SharedString::from(format!("look-reset-button-{slot:?}"))).label("Reset to the standard page").small().secondary().disabled(standard).tooltip(tip).on_click(cx.listener(move |this, _, window, cx| {
                        match slot {
                            Slot::Librarian => {
                                this.set_librarian_look(Default::default(), cx);
                                this.looks.github_typed.clear();
                                for field in [this.looks.greeting_name.clone(), this.looks.github_name.clone()].into_iter().flatten() {
                                    field.update(cx, |f, cx| f.set_value("", window, cx));
                                }
                            }
                            _ => this.set_scheduled_look(Default::default(), cx),
                        }
                        this.forget_fade(slot);
                    })),
                ),
            )
            .into_any_element()
    }

    // Controls.

    fn look_switch(&self, id: &'static str, on: bool, tip: &'static str, disabled: bool, cx: &mut Context<Self>, set: impl Fn(&mut Self, bool, &mut Window, &mut Context<Self>) + 'static) -> AnyElement {
        self.look_switch_owned(id.to_owned(), on, tip.to_owned(), disabled, cx, set)
    }

    fn look_switch_owned(&self, id: String, on: bool, tip: String, disabled: bool, cx: &mut Context<Self>, set: impl Fn(&mut Self, bool, &mut Window, &mut Context<Self>) + 'static) -> AnyElement {
        let me = cx.entity().downgrade();
        let set = Rc::new(set);
        div()
            .id(SharedString::from(format!("{id}-holder")))
            .test_support()
            .a11y_role(Role::GenericContainer)
            .aria_label(tip.clone())
            .tip(tip)
            .flex_none()
            .child(MacSwitch::new(SharedString::from(id), on).disabled(disabled).on_change(move |on, window, cx| {
                let set = set.clone();
                me.update(cx, |this, cx| set(this, on, window, cx)).ok();
            }))
            .into_any_element()
    }

    /// Top, middle or bottom of the page.
    fn place_menu(&self, id: String, place: Place, disabled: bool, cx: &mut Context<Self>, set: impl Fn(&mut Self, Place, &mut Context<Self>) + 'static) -> AnyElement {
        let me = cx.entity().downgrade();
        let set = Rc::new(set);
        let button = Button::new(SharedString::from(id.clone())).label(place.title()).small().secondary().dropdown_caret(true).disabled(disabled).tooltip("Top, middle or bottom of the page").dropdown_menu(move |menu: PopupMenu, _, _| {
            let mut menu = menu;
            for choice in Place::ALL {
                let (me, set) = (me.clone(), set.clone());
                menu = menu.item(PopupMenuItem::new(choice.title()).checked(choice == place).on_click(move |_, _, cx| {
                    let set = set.clone();
                    me.update(cx, |this, cx| set(this, choice, cx)).ok();
                }));
            }
            menu
        });
        div().id(SharedString::from(format!("{id}-holder"))).test_support().a11y_role(Role::GenericContainer).aria_label("Top, middle or bottom of the page").flex_none().w(px(92.)).flex().justify_end().child(button).into_any_element()
    }

    /// The greeting's name or the GitHub user name: made once with what's
    /// saved, then kept as it's typed.
    fn look_field(&mut self, field: Field, now: &str, placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let slot = match field {
            Field::Greeting => &self.looks.greeting_name,
            Field::GitHub => &self.looks.github_name,
        };
        let state = match slot {
            Some(state) => state.clone(),
            None => {
                let start = now.to_owned();
                let state = cx.new(|cx| {
                    let mut s = InputState::new(window, cx).placeholder(placeholder);
                    s.set_value(start, window, cx);
                    s
                });
                let sub = cx.subscribe(&state, move |this, state, ev: &InputEvent, cx| {
                    if !matches!(ev, InputEvent::Change) {
                        return;
                    }
                    let typed = state.read(cx).value().to_string();
                    let mut look = this.librarian_look();
                    match field {
                        Field::Greeting => look.greeting_name = typed,
                        Field::GitHub => {
                            // Kept as it's typed once it's a real name, or cleared.
                            let clean = rules::github_user(&typed);
                            this.looks.github_typed = typed.clone();
                            if typed.trim().is_empty() || !clean.is_empty() {
                                look.github_user = clean;
                            }
                        }
                    }
                    this.set_librarian_look(look, cx);
                });
                self.looks.keep(sub);
                if field == Field::GitHub {
                    self.looks.github_typed = now.to_owned();
                    self.looks.github_name = Some(state.clone());
                } else {
                    self.looks.greeting_name = Some(state.clone());
                }
                state
            }
        };
        div().w(px(150.)).child(Input::new(&state).small()).into_any_element()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Greeting,
    GitHub,
}

/// The Mac account's full name ("Wayne Sutton"), for the greeting's first
/// suggestion. Empty when it can't be read.
fn full_user_name() -> String {
    std::process::Command::new("/usr/bin/id").arg("-F").output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned()).unwrap_or_default()
}

/// A section of the panel: its heading (with more behind an ⓘ) over its
/// rows in a rounded group, as the Settings form draws them.
fn section(title: &'static str, about: Option<&'static str>, rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    let line = style::primary_opacity(cx, 0.05);
    let mut group = v_flex();
    let count = rows.len();
    for (i, r) in rows.into_iter().enumerate() {
        group = group.child(r);
        if i + 1 < count {
            group = group.child(div().h(gpui_kit::px(1.)).mx(px(10.)).bg(line));
        }
    }
    v_flex()
        .gap(px(8.))
        .child(h_flex().gap(px(6.)).items_center().px(px(10.)).child(div().text_size(px(13.)).font_semibold().child(title)).children(about.map(|a| info_tip(SharedString::from(format!("look-about-{title}")), a, cx))))
        .child(div().rounded(px(10.)).bg(style::primary_opacity(cx, 0.03)).child(group))
        .into_any_element()
}

fn row(label: &str, control: impl IntoElement, _cx: &App) -> AnyElement {
    h_flex().min_h(px(34.)).px(px(10.)).py(px(5.)).gap(px(10.)).items_center().text_size(px(13.)).child(div().flex_1().min_w_0().truncate().child(label.to_owned())).child(control).into_any_element()
}

/// A Layout row: the name, then the switch and the place in columns of their
/// own, so they line up whatever the name is.
fn layout_row(label: &str, switch: AnyElement, place: AnyElement, _cx: &App) -> AnyElement {
    h_flex().min_h(px(34.)).px(px(10.)).py(px(5.)).gap(px(10.)).items_center().text_size(px(13.)).child(div().flex_1().min_w_0().truncate().child(label.to_owned())).child(switch).child(place).into_any_element()
}

fn note(text: &str, problem: bool, cx: &App) -> AnyElement {
    let color = if problem { gpui_kit::component::ActiveTheme::theme(cx).danger } else { style::secondary_text(cx) };
    div().px(px(10.)).py(px(6.)).text_size(px(style::size::LABEL)).text_color(color).child(text.to_owned()).into_any_element()
}

fn tipped_field(id: &'static str, tip: &'static str, field: AnyElement) -> AnyElement {
    div().id(id).tip(tip).child(field).into_any_element()
}

fn small_button(id: String, label: &'static str, tip: &'static str, cx: &mut Context<Threadbear>, action: impl Fn(&mut Threadbear, &mut Context<Threadbear>) + 'static) -> AnyElement {
    div()
        .id(SharedString::from(format!("{id}-holder")))
        .test_support()
        .a11y_role(Role::GenericContainer)
        .aria_label(tip)
        .flex_none()
        .child(Button::new(SharedString::from(id)).label(label).small().secondary().tooltip(tip).on_click(cx.listener(move |this, _, _, cx| action(this, cx))))
        .into_any_element()
}
