// Skills (the bar's Skills, while the skillsStore experiment is on), a port
// of the Swift app's SkillsPage: Threadbear's own skills, which every
// thread's agent gets on every login, Claude and Codex, over the main panel
// as Notes and Settings are. Import one from a GitHub link, write one with
// New Skill, add a suggested one; Always on means agents use it whenever it
// fits, off means only its slash command runs it.
//
// The list lives on the runner (runner/src/skills.ts, docs/protocol.md,
// "Skills"): `skills.list` at connect, `skills.updated` after any change,
// from this app, the Swift one or another thread. The page keeps only what's
// being typed.
use crate::a11y::A11yRole as _;
use crate::app::Threadbear;
use crate::highlights::{tipped, tipped_as};
use crate::mac_switch::MacSwitch;
use crate::style::{self, px};
use crate::tips::Tip as _;
use crate::{call, icons};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Disableable as _, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::HashSet;
use threadbear_core::skills::{self, Skill, Suggested};

actions!(skills_page, [CloseSkills]);

/// What Your skills says when the search leaves none.
fn no_match(typed: &str) -> String {
    format!(
        "No skill's name has \u{201C}{}\u{201D} in it.",
        typed.trim()
    )
}

const PAGE_CONTEXT: &str = "SkillsPage";
/// The column's width at 100% (`.frame(maxWidth: 680)`).
const COLUMN: f32 = 680.;

fn bind_keys(cx: &mut App) {
    static BOUND: std::sync::Once = std::sync::Once::new();
    BOUND.call_once(|| cx.bind_keys([KeyBinding::new("escape", CloseSkills, Some(PAGE_CONTEXT))]));
}

/// What the runner says, kept on the app so the bar knows whether to show
/// Skills before the page is ever opened.
#[derive(Default)]
pub(crate) struct SkillsState {
    pub(crate) skills: Vec<Skill>,
    pub(crate) suggested: Vec<Suggested>,
    /// The runner answered `skills.list` (an older one has no store).
    pub(crate) supported: bool,
    /// Names with a change on its way (Always on, Update, Remove, Add), so
    /// a second click can't send it twice.
    pub(crate) busy: HashSet<String>,
}

impl Threadbear {
    /// The skillsStore experiment is on for this Mac's runner. Settings'
    /// switch first, as it changes without a new hello, else hello's list.
    pub(crate) fn skills_store_on(&self) -> bool {
        self.experiment_on(skills::EXPERIMENT)
    }

    /// The bar's button: an older runner has no store, and the experiment
    /// off hides it (the runner keeps the skills).
    pub(crate) fn skills_shown(&self) -> bool {
        self.skills.supported && self.skills_store_on()
    }

    /// At connect, with the other lists.
    pub(crate) fn load_skills(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = call(client, "skills.list", json!({})).await;
            this.update(cx, |this, cx| {
                match res {
                    Ok(v) => {
                        this.skills.supported = true;
                        this.receive_skills(&v);
                    }
                    // A runner from before Skills, or one that refused: no button.
                    Err(_) => this.skills.supported = false,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// `skills.list`'s answer, or a `skills.updated` (which has no suggestions).
    pub(crate) fn receive_skills(&mut self, v: &Value) {
        self.skills.skills = skills::decode_list(&v["skills"]);
        if v.get("suggested").is_some() {
            self.skills.suggested = skills::decode_suggested(&v["suggested"]);
        } else {
            let have: HashSet<&str> = self.skills.skills.iter().map(|s| s.name.as_str()).collect();
            self.skills
                .suggested
                .retain(|s| !have.contains(s.name.as_str()));
        }
        if !self.skills_shown() {
            self.skills_page = None;
        }
    }

    /// `skills.updated`: any change, from here or anywhere else. Asks for
    /// the list again too, for the suggestions a removal brings back.
    pub(crate) fn skills_updated(&mut self, params: &Value, cx: &mut Context<Self>) {
        self.skills.supported = true;
        self.receive_skills(params);
        self.load_skills(cx);
    }

    pub(crate) fn toggle_skills(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.close_skills(cx) {
            self.open_skills(window, cx);
        }
    }

    /// Skills over the main panel, in place of Settings or Notes if up.
    pub(crate) fn open_skills(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.skills_shown() || self.skills_page.is_some() {
            return;
        }
        self.close_tasks(cx);
        self.close_settings(cx);
        self.close_notes(cx);
        let app = cx.entity().downgrade();
        self.skills_page = Some(cx.new(|cx| SkillsPage::new(app, window, cx)));
        self.load_skills(cx);
        self.sync_menus(cx);
        cx.notify();
    }

    /// False if it wasn't up.
    pub(crate) fn close_skills(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(page) = self.skills_page.take() else {
            return false;
        };
        let held = page.read(cx).focus_handles(cx);
        let (window, focus) = (self.main_window, self.root_focus.clone());
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    if window.focused(cx).is_none() || held.iter().any(|h| h.is_focused(window)) {
                        window.focus(&focus, cx);
                        crate::web_panels::keyboard_to_app(window);
                    }
                })
                .ok();
        });
        self.sync_menus(cx);
        cx.notify();
        true
    }

    pub(crate) fn render_skills_page(&self) -> AnyElement {
        match &self.skills_page {
            Some(page) => div().size_full().child(page.clone()).into_any_element(),
            None => div().into_any_element(),
        }
    }

    /// Always on, Update to the Latest or Remove: one request, the row busy
    /// until it lands, the error in the bar if the runner says no.
    fn change_skill(
        &mut self,
        name: String,
        method: &'static str,
        params: Value,
        cx: &mut Context<Self>,
    ) {
        if !self.skills.busy.insert(name.clone()) {
            return;
        }
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = call(client, method, params).await;
            this.update(cx, |this, cx| {
                this.skills.busy.remove(&name);
                match res {
                    Ok(v) if v.get("skills").is_some() => this.receive_skills(&v),
                    Ok(_) => {}
                    Err(e) => this.show_error(e.message, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// A guess at once (the switch moves), put back if the runner says no.
    fn set_skill_always(&mut self, name: String, always: bool, cx: &mut Context<Self>) {
        let before = self
            .skills
            .skills
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.always);
        if let Some(skill) = self.skills.skills.iter_mut().find(|s| s.name == name) {
            skill.always = always;
        }
        if self.skills.busy.contains(&name) {
            return;
        }
        self.skills.busy.insert(name.clone());
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = call(
                client,
                "skills.update",
                json!({ "name": name, "always": always }),
            )
            .await;
            this.update(cx, |this, cx| {
                this.skills.busy.remove(&name);
                match res {
                    Ok(v) => this.receive_skills(&v),
                    Err(e) => {
                        if let (Some(was), Some(skill)) = (
                            before,
                            this.skills.skills.iter_mut().find(|s| s.name == name),
                        ) {
                            skill.always = was;
                        }
                        this.show_error(e.message, cx);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The dump's `skills`: the button, the page, the rows and the form.
    pub(crate) fn skills_dump(&self, cx: &App) -> Value {
        let page = self.skills_page.as_ref().map(|p| p.read(cx));
        json!({
            "button": self.skills_shown(),
            "supported": self.skills.supported,
            "experiment": self.skills_store_on(),
            "open": page.is_some(),
            "skills": self.skills.skills.iter().map(|s| json!({ "name": s.name, "always": s.always, "source": s.source_line() })).collect::<Vec<_>>(),
            "suggested": self.skills.suggested.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
            "form": page.and_then(|p| p.form.as_ref()).map(|f| json!({ "replacing": f.replacing, "name": f.name.read(cx).value().to_string(), "error": f.error })),
            "importError": page.and_then(|p| p.import_error.clone()),
            "search": page.map(|p| p.search.read(cx).value().to_string()),
            "noMatch": page.and_then(|p| {
                let typed = p.search.read(cx).value().to_string();
                (!self.skills.skills.is_empty() && !self.skills.skills.iter().any(|s| skills::name_matches(&s.name, &typed))).then(|| no_match(&typed))
            }),
            "suggestedShown": page.map(|p| {
                let typed = p.search.read(cx).value().to_string();
                self.skills.suggested.iter().filter(|s| skills::name_matches(&s.name, &typed)).map(|s| s.name.clone()).collect::<Vec<_>>()
            }),
            // What the search leaves showing, in order.
            "shown": page.map(|p| {
                let typed = p.search.read(cx).value().to_string();
                self.skills.skills.iter().filter(|s| skills::name_matches(&s.name, &typed)).map(|s| s.name.clone()).collect::<Vec<_>>()
            }),
        })
    }
}

/// New Skill, or Edit on one he wrote here (`replacing`).
struct Form {
    replacing: Option<String>,
    name: Entity<InputState>,
    description: Entity<InputState>,
    text: Entity<TextareaState>,
    saving: bool,
    error: Option<String>,
}

pub(crate) struct SkillsPage {
    app: WeakEntity<Threadbear>,
    link: Entity<InputState>,
    /// Search over Your skills and Suggested, by name.
    search: Entity<InputState>,
    importing: bool,
    import_error: Option<String>,
    form: Option<Form>,
    scroll: ScrollHandle,
    focus: FocusHandle,
    _subs: Vec<Subscription>,
}

impl SkillsPage {
    fn new(app: WeakEntity<Threadbear>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        bind_keys(cx);
        let link = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("A GitHub link to a skill or a repo of skills, or a page that has one")
        });
        let subs = vec![
            cx.subscribe_in(&link, window, |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. }) {
                    this.run_import(window, cx);
                }
            }),
        ];
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search skills by name"));
        let mut subs = subs;
        subs.push(cx.subscribe(&search, |_, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Change) {
                cx.notify();
            }
        }));
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        crate::web_panels::keyboard_to_app(window);
        SkillsPage {
            app,
            link,
            search,
            importing: false,
            import_error: None,
            form: None,
            scroll: ScrollHandle::new(),
            focus,
            _subs: subs,
        }
    }

    fn focus_handles(&self, cx: &App) -> Vec<FocusHandle> {
        let mut out = vec![
            self.focus.clone(),
            self.link.read(cx).focus_handle(cx),
            self.search.read(cx).focus_handle(cx),
        ];
        if let Some(f) = &self.form {
            out.extend([
                f.name.read(cx).focus_handle(cx),
                f.description.read(cx).focus_handle(cx),
                f.text.read(cx).focus_handle(cx),
            ]);
        }
        out
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        let app = self.app.clone();
        cx.defer(move |cx| {
            app.update(cx, |app, cx| {
                app.close_skills(cx);
            })
            .ok();
        });
    }

    fn run_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.link.read(cx).value().trim().to_owned();
        if url.is_empty() || self.importing {
            return;
        }
        let Some(client) = self.app.upgrade().map(|a| a.read(cx).client.clone()) else {
            return;
        };
        self.importing = true;
        self.import_error = None;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let res = call(client, "skills.add", json!({ "url": url })).await;
            this.update_in(cx, |this, window, cx| {
                this.importing = false;
                match res {
                    Ok(v) => {
                        this.link.update(cx, |l, cx| l.set_value("", window, cx));
                        this.app
                            .update(cx, |app, cx| {
                                app.receive_skills(&v);
                                app.load_skills(cx);
                            })
                            .ok();
                    }
                    Err(e) => this.import_error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn add_suggested(&mut self, id: String, cx: &mut Context<Self>) {
        self.app
            .update(cx, |app, cx| {
                app.change_skill(id.clone(), "skills.add", json!({ "suggested": id }), cx)
            })
            .ok();
    }

    /// New Skill (no name) or Edit, its text read from the runner first.
    fn open_form(&mut self, editing: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("name, like brand-voice"));
        let description = cx.new(|cx| {
            InputState::new(window, cx).placeholder(
                "When should agents use it? (optional: its first line is used otherwise)",
            )
        });
        let text = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(
                    "Paste the skill's text here: what the agent should do, or a whole SKILL.md",
                )
                .auto_grow(8, 16)
        });
        if let Some(n) = &editing {
            name.update(cx, |s, cx| s.set_value(n.clone(), window, cx));
        }
        name.update(cx, |s, cx| s.focus(window, cx));
        crate::web_panels::keyboard_to_app(window);
        self.form = Some(Form {
            replacing: editing.clone(),
            name,
            description,
            text,
            saving: false,
            error: None,
        });
        cx.notify();
        let Some(editing) = editing else { return };
        let Some(client) = self.app.upgrade().map(|a| a.read(cx).client.clone()) else {
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            let res = call(client, "skills.read", json!({ "name": editing })).await;
            this.update_in(cx, |this, window, cx| {
                let Some(form) = this
                    .form
                    .as_mut()
                    .filter(|f| f.replacing.as_deref() == Some(editing.as_str()))
                else {
                    return;
                };
                match res {
                    Ok(v) => {
                        let body = v["text"].as_str().unwrap_or("").to_owned();
                        form.text.update(cx, |t, cx| t.set_value(body, window, cx));
                    }
                    Err(e) => form.error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn save_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let slug = skills::slug(&form.name.read(cx).value());
        let text = form.text.read(cx).value().to_string();
        if slug.is_empty() || text.trim().is_empty() || form.saving {
            return;
        }
        let Some(client) = self.app.upgrade().map(|a| a.read(cx).client.clone()) else {
            return;
        };
        let description = form.description.read(cx).value().trim().to_owned();
        let mut params = json!({ "name": slug, "text": text });
        if !description.is_empty() {
            params["description"] = json!(description);
        }
        if let Some(r) = &form.replacing {
            params["replacing"] = json!(r);
        }
        form.saving = true;
        form.error = None;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let res = call(client, "skills.save", params).await;
            this.update_in(cx, |this, _, cx| {
                match res {
                    Ok(v) => {
                        this.form = None;
                        this.app
                            .update(cx, |app, cx| {
                                app.receive_skills(&v);
                                app.load_skills(cx);
                            })
                            .ok();
                    }
                    Err(e) => {
                        if let Some(f) = this.form.as_mut() {
                            f.saving = false;
                            f.error = Some(e.message);
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // Drawing.

    fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        let me = cx.entity().downgrade();
        let new = crate::push_button::PushButton::new(
            "skills-new",
            "New Skill",
            "Write a skill of your own, with its own slash command",
        )
        .disabled(self.form.is_some())
        .render(cx, move |_, window, cx| {
            me.update(cx, |this, cx| this.open_form(None, window, cx))
                .ok();
        });
        let close_fill = style::primary_opacity(cx, 0.08);
        let close_hover = style::primary_opacity(cx, 0.14);
        let close = tipped(
            div()
                .id("skills-close")
                .size(px(28.))
                .rounded_full()
                .bg(close_fill)
                .hover(move |s| s.bg(close_hover))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer(),
            "Close Skills (Esc)",
        )
        .on_click(cx.listener(|this, _, _, cx| this.close(cx)))
        .child(icons::symbol("xmark").with_size(px(12.)));
        h_flex()
            .flex_none()
            .gap(px(10.))
            .pl(px(20.))
            .pr(px(28.))
            .py(px(10.))
            .items_center()
            .border_b(gpui_kit::px(0.5))
            .border_color(style::hairline(cx))
            .child(
                div()
                    .text_size(px(17.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Skills"),
            )
            .child(div().flex_1())
            .child(new)
            .child(close)
            .into_any_element()
    }

    fn heading(text: &'static str) -> Div {
        div()
            .text_size(px(style::size::LABEL))
            .font_weight(FontWeight::SEMIBOLD)
            .child(text)
    }

    fn import_box(&self, cx: &mut Context<Self>) -> AnyElement {
        let empty = self.link.read(cx).value().trim().is_empty();
        let me = cx.entity().downgrade();
        let import = crate::push_button::PushButton::new(
            "skills-import",
            "Import",
            "Fetch it from GitHub, pinned to its current commit",
        )
        .busy(self.importing)
        .disabled(self.importing || empty)
        .render(cx, move |_, window, cx| {
            me.update(cx, |this, cx| this.run_import(window, cx)).ok();
        });
        let field = tipped_as(
            div().id("skills-link").flex_1().min_w_0(),
            "Paste github.com/owner/repo, a folder or a SKILL.md link, or a skill's own page",
            gpui_kit::Role::GenericContainer,
        )
        .child(Input::new(&self.link).disabled(self.importing));
        let mut col = v_flex()
            .gap(px(6.))
            .child(Self::heading("Import a skill"))
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(field)
                    .child(import),
            );
        if let Some(err) = &self.import_error {
            col = col.child(
                div()
                    .text_size(px(style::size::LABEL))
                    .text_color(style::failed_text(cx))
                    .child(err.clone()),
            );
        }
        col.into_any_element()
    }

    fn form(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let form = self.form.as_ref()?;
        let secondary = style::secondary_text(cx);
        let slug = skills::slug(&form.name.read(cx).value());
        let typed = form.name.read(cx).value().trim().to_lowercase();
        let can_save =
            !slug.is_empty() && !form.text.read(cx).value().trim().is_empty() && !form.saving;
        let title = match &form.replacing {
            Some(r) => format!("Edit {r}"),
            None => "New skill".to_owned(),
        };
        let name_row = h_flex()
            .gap(px(4.))
            .items_center()
            .child(
                div()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(secondary)
                    .child("/"),
            )
            .child(
                tipped_as(
                    div().id("skills-form-name").flex_1().min_w_0(),
                    format!(
                        "Its slash command: type /{} in a message to use it",
                        if slug.is_empty() { "name" } else { &slug }
                    ),
                    gpui_kit::Role::GenericContainer,
                )
                .child(Input::new(&form.name).font_family(cx.theme().mono_font_family.clone())),
            );
        let mut col = v_flex()
            .id("skills-form")
            .gap(px(10.))
            .p(px(14.))
            .rounded(px(10.))
            .border_1()
            .border_color(style::control_accent(cx).opacity(0.5))
            .child(
                div()
                    .text_size(px(style::size::LABEL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(name_row);
        if form.replacing.is_none() {
            col = col.child(
                tipped_as(
                    div().id("skills-form-description"),
                    "What it's for, so agents pick it up by themselves when Always on",
                    gpui_kit::Role::GenericContainer,
                )
                .child(Input::new(&form.description)),
            );
        }
        col = col.child(
            tipped_as(
                div()
                    .id("skills-form-text")
                    .p(px(6.))
                    .rounded(px(6.))
                    .bg(style::primary_opacity(cx, 0.04)),
                "The skill's instructions. A pasted SKILL.md keeps its own header",
                gpui_kit::Role::GenericContainer,
            )
            .child(
                crate::composer::bare_textarea(Textarea::new(&form.text))
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(px(style::size::CODE)),
            ),
        );
        if let Some(err) = &form.error {
            col = col.child(
                div()
                    .text_size(px(style::size::LABEL))
                    .text_color(style::failed_text(cx))
                    .child(err.clone()),
            );
        }
        let (cancel_me, save_me) = (cx.entity().downgrade(), cx.entity().downgrade());
        let cancel = crate::push_button::PushButton::new(
            "skills-form-cancel",
            "Cancel",
            "Close without saving",
        )
        .render(cx, move |_, _, cx| {
            cancel_me
                .update(cx, |this, cx| {
                    this.form = None;
                    cx.notify();
                })
                .ok();
        });
        let save = Button::new("skills-form-save")
            .label("Save")
            .primary()
            .small()
            .disabled(!can_save)
            .loading(form.saving)
            .tooltip("Save it")
            .on_click(move |_, window, cx| {
                save_me
                    .update(cx, |this, cx| this.save_form(window, cx))
                    .ok();
            });
        let saved_as = (!slug.is_empty() && slug != typed).then(|| {
            div()
                .text_size(px(style::size::SMALL))
                .text_color(secondary)
                .child(format!("Saved as /{slug}"))
        });
        col = col.child(
            h_flex()
                .gap(px(8.))
                .items_center()
                .children(saved_as)
                .child(div().flex_1())
                .child(cancel)
                .child(save),
        );
        Some(col.into_any_element())
    }

    fn skill_row(
        &self,
        skill: &Skill,
        first: bool,
        busy: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let secondary = style::secondary_text(cx);
        let tertiary = style::tertiary_label(cx);
        let name = skill.name.clone();
        let chip = tipped_as(
            div()
                .id(SharedString::from(format!("skill-slash-{name}")))
                .px(px(5.))
                .rounded_full()
                .bg(style::primary_opacity(cx, 0.07)),
            format!("Type /{name} in any thread to use it"),
            gpui_kit::Role::GenericContainer,
        )
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(px(style::size::SMALL))
        .text_color(secondary)
        .child(format!("/{name}"));
        let text = v_flex()
            .flex_1()
            .min_w_0()
            .gap(px(3.))
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(name.clone()))
                    .child(chip),
            )
            .when(!skill.description.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(px(style::size::LABEL))
                        .text_color(secondary)
                        .line_clamp(3)
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(skill.description.clone()),
                )
            })
            .child(
                div()
                    .text_size(px(style::size::SMALL))
                    .text_color(tertiary)
                    .child(skill.source_line()),
            );
        let tip = if skill.always {
            format!("Agents use it whenever it fits. Turn off to use it only with /{name}")
        } else {
            format!("Only /{name} uses it. Turn on to let agents use it whenever it fits")
        };
        let app = self.app.clone();
        let switch_name = name.clone();
        let switch = div()
            .id(SharedString::from(format!("skill-always-{name}")))
            .test_support()
            .a11y_role(gpui_kit::Role::GenericContainer)
            .aria_label(tip.clone())
            .tip(tip)
            .child(
                h_flex()
                    .gap(px(6.))
                    .items_center()
                    .child(
                        div()
                            .text_size(px(style::size::SMALL))
                            .text_color(secondary)
                            .child("Always on"),
                    )
                    .child(
                        MacSwitch::new(
                            SharedString::from(format!("skill-always-switch-{name}")),
                            skill.always,
                        )
                        .outside_form()
                        .disabled(busy)
                        .on_change(move |on, _, cx| {
                            app.update(cx, |app, cx| {
                                app.set_skill_always(switch_name.clone(), on, cx)
                            })
                            .ok();
                        }),
                    ),
            );
        let more = self.more_menu(skill, busy, cx);
        h_flex()
            .id(SharedString::from(format!("skill-row-{name}")))
            .test_support()
            .role(gpui_kit::Role::ListItem)
            .aria_label(SharedString::from(name))
            .items_start()
            .gap(px(12.))
            .px(px(14.))
            .py(px(10.))
            .when(!first, |d| {
                d.border_t(gpui_kit::px(0.5))
                    .border_color(style::hairline(cx))
            })
            .child(text)
            .child(switch)
            .child(more)
            .into_any_element()
    }

    fn more_menu(&self, skill: &Skill, busy: bool, cx: &mut Context<Self>) -> AnyElement {
        let (app, page) = (self.app.clone(), cx.entity().downgrade());
        let s = skill.clone();
        let button = Button::new(SharedString::from(format!("skill-more-{}", s.name)))
            .icon(icons::symbol("ellipsis"))
            .ghost()
            .small()
            .loading(busy)
            .tooltip(format!("More for {}", s.name))
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                let name = s.name.clone();
                if s.is_custom() {
                    let (page, n) = (page.clone(), name.clone());
                    menu = menu.item(
                        PopupMenuItem::new("Edit")
                            .icon(icons::symbol("pencil"))
                            .on_click(move |_, window, cx| {
                                page.update(cx, |p, cx| p.open_form(Some(n.clone()), window, cx))
                                    .ok();
                            }),
                    );
                } else {
                    let (app, n) = (app.clone(), name.clone());
                    menu = menu.item(
                        PopupMenuItem::new("Update to the Latest")
                            .icon(icons::symbol("arrow.down.circle"))
                            .on_click(move |_, _, cx| {
                                app.update(cx, |a, cx| {
                                    a.change_skill(
                                        n.clone(),
                                        "skills.update",
                                        json!({ "name": n, "latest": true }),
                                        cx,
                                    )
                                })
                                .ok();
                            }),
                    );
                }
                if let Some(url) = s.source_url() {
                    menu = menu.item(
                        PopupMenuItem::new("View on GitHub")
                            .icon(icons::symbol("safari"))
                            .on_click(move |_, _, cx| {
                                crate::open_gate::open_web(&url, cx);
                            }),
                    );
                }
                let file = std::path::Path::new(&s.dir)
                    .join("SKILL.md")
                    .to_string_lossy()
                    .into_owned();
                menu = menu.item(
                    PopupMenuItem::new("Show in Finder")
                        .icon(icons::symbol("folder"))
                        .on_click(move |_, _, cx| {
                            crate::open_gate::reveal(&file, crate::open_gate::Origin::ThisMac, cx);
                        }),
                );
                let (app, n) = (app.clone(), name.clone());
                menu.separator().item(
                    PopupMenuItem::new("Remove")
                        .icon(icons::symbol("trash"))
                        .on_click(move |_, _, cx| {
                            app.update(cx, |a, cx| {
                                a.change_skill(n.clone(), "skills.remove", json!({ "name": n }), cx)
                            })
                            .ok();
                        }),
                )
            });
        div().flex_none().child(button).into_any_element()
    }

    /// The search, beside Your skills' heading: a magnifying glass, the
    /// field, and its clear button, as Settings > Keyboard's.
    fn search_box(&self, cx: &mut Context<Self>) -> AnyElement {
        let faint = style::tertiary_label(cx);
        tipped_as(
            div().id("skills-search"),
            "Search skills by name",
            gpui_kit::Role::GenericContainer,
        )
        .w(px(240.))
        .h(px(26.))
        .pl(px(8.))
        .pr(px(6.))
        .flex()
        .items_center()
        .gap(px(6.))
        .rounded(px(7.))
        .bg(style::primary_opacity(cx, 0.035))
        .border(gpui_kit::px(0.5))
        .border_color(style::hairline(cx))
        .child(
            div().flex_none().child(
                icons::symbol("magnifyingglass")
                    .with_size(px(11.5))
                    .text_color(faint),
            ),
        )
        .child(
            div().flex_1().min_w_0().child(
                Input::new(&self.search)
                    .appearance(false)
                    .small()
                    .cleanable(true)
                    .pl(px(0.))
                    .text_size(px(style::size::LABEL)),
            ),
        )
        .into_any_element()
    }

    fn installed(
        &self,
        all: &[Skill],
        typed: &str,
        busy: &HashSet<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let secondary = style::secondary_text(cx);
        let mut col = v_flex().gap(px(8.)).child(
            h_flex()
                .items_center()
                .gap(px(12.))
                .child(Self::heading("Your skills"))
                .child(div().flex_1())
                .when(!all.is_empty(), |d| d.child(self.search_box(cx))),
        );
        if all.is_empty() {
            return col
                .child(
                    div()
                        .text_size(px(style::size::LABEL))
                        .text_color(secondary)
                        .child("None yet. Import one above, or add a suggested one below."),
                )
                .into_any_element();
        }
        let list: Vec<&Skill> = all
            .iter()
            .filter(|s| skills::name_matches(&s.name, typed))
            .collect();
        if list.is_empty() {
            return col
                .child(
                    div()
                        .text_size(px(style::size::LABEL))
                        .text_color(secondary)
                        .child(no_match(typed)),
                )
                .into_any_element();
        }
        let rows: Vec<AnyElement> = list
            .iter()
            .enumerate()
            .map(|(i, s)| self.skill_row(s, i == 0, busy.contains(&s.name), cx))
            .collect();
        col = col
            .child(
                v_flex()
                    .id("skills-list")
                    .rounded(px(10.))
                    .bg(style::primary_opacity(cx, 0.04))
                    .children(rows),
            )
            .child(
                div()
                    .text_size(px(style::size::SMALL))
                    .text_color(style::tertiary_label(cx))
                    .child("A change reaches each thread when its agent next starts."),
            );
        col.into_any_element()
    }

    fn suggested(
        &self,
        all: &[Suggested],
        typed: &str,
        busy: &HashSet<String>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let list: Vec<&Suggested> = all
            .iter()
            .filter(|s| skills::name_matches(&s.name, typed))
            .collect();
        if list.is_empty() {
            return None;
        }
        let secondary = style::secondary_text(cx);
        let mut col = v_flex().gap(px(8.)).child(Self::heading("Suggested"));
        for s in list {
            let me = cx.entity().downgrade();
            let id = s.id.clone();
            let add = crate::push_button::PushButton::new(
                SharedString::from(format!("skill-add-{}", s.id)),
                "Add",
                format!("Add {} to your skills", s.name),
            )
            .busy(busy.contains(&s.id))
            .disabled(busy.contains(&s.id))
            .render(cx, move |_, _, cx| {
                me.update(cx, |this, cx| this.add_suggested(id.clone(), cx))
                    .ok();
            });
            col = col.child(
                h_flex()
                    .id(SharedString::from(format!("skill-suggested-{}", s.id)))
                    .items_start()
                    .gap(px(12.))
                    .px(px(14.))
                    .py(px(10.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(style::primary_opacity(cx, 0.08))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(3.))
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(s.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(style::size::LABEL))
                                    .text_color(secondary)
                                    .line_clamp(3)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .child(s.description.clone()),
                            ),
                    )
                    .child(add),
            );
        }
        Some(col.into_any_element())
    }
}

impl Render for SkillsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (list, suggested, busy) = self
            .app
            .upgrade()
            .map(|a| {
                let s = &a.read(cx).skills;
                (s.skills.clone(), s.suggested.clone(), s.busy.clone())
            })
            .unwrap_or_default();
        let typed = self.search.read(cx).value().to_string();
        let intro = div()
            .text_size(px(style::size::LABEL))
            .text_color(style::secondary_text(cx))
            .child("Skills give every thread's agent extra know-how, on every login, Claude and Codex alike. With Always on, agents use a skill whenever it fits; with it off, only when you type its slash command, like /pele.");
        let column = v_flex()
            .w_full()
            .max_w(px(COLUMN))
            .gap(px(22.))
            .child(intro)
            .children(self.form(cx))
            .child(self.import_box(cx))
            .child(self.installed(&list, &typed, &busy, cx))
            .children(self.suggested(&suggested, &typed, &busy, cx));
        v_flex()
            .id("skills-page")
            .key_context(PAGE_CONTEXT)
            .track_focus(&self.focus)
            .size_full()
            .bg(cx.theme().background)
            .on_action(cx.listener(|this, _: &CloseSkills, _, cx| this.close(cx)))
            .child(self.header(cx))
            .child(
                div()
                    .id("skills-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        h_flex()
                            .w_full()
                            .justify_center()
                            .px(px(28.))
                            .py(px(20.))
                            .child(column),
                    ),
            )
    }
}

/// The automation's `skills` command (features/skills-store.feature):
/// Skills opened, or `close`; `new` opens New Skill with `name` and `text`
/// typed, and `save` saves it; `always` with `name` flips that row's switch.
#[cfg(feature = "automation")]
pub(crate) fn automation(
    app: &Entity<Threadbear>,
    cmd: &Value,
    window: &mut Window,
    cx: &mut App,
) -> Result<Value, String> {
    if cmd["close"].as_bool() == Some(true) {
        app.update(cx, |t, cx| t.close_skills(cx));
        return Ok(json!("ok closed"));
    }
    if app.read(cx).skills_page.is_none() {
        app.update(cx, |t, cx| t.open_skills(window, cx));
    }
    let Some(page) = app.read(cx).skills_page.clone() else {
        return Err(
            "Skills is hidden: the skillsStore experiment is off, or the runner has no store"
                .into(),
        );
    };
    if cmd["new"].as_bool() == Some(true) {
        page.update(cx, |p, cx| {
            p.open_form(None, window, cx);
            if let Some(f) = &p.form {
                if let Some(n) = cmd["name"].as_str() {
                    f.name
                        .update(cx, |s, cx| s.set_value(n.to_owned(), window, cx));
                }
                if let Some(t) = cmd["text"].as_str() {
                    f.text
                        .update(cx, |s, cx| s.set_value(t.to_owned(), window, cx));
                }
            }
        });
    }
    if let Some(typed) = cmd["search"].as_str() {
        // Setting the text isn't typing, so the page is told to draw again.
        page.update(cx, |p, cx| {
            p.search
                .update(cx, |s, cx| s.set_value(typed.to_owned(), window, cx));
            cx.notify();
        });
    }
    if cmd["save"].as_bool() == Some(true) {
        page.update(cx, |p, cx| p.save_form(window, cx));
    }
    if let (Some(name), Some(on)) = (cmd["name"].as_str(), cmd["always"].as_bool()) {
        app.update(cx, |t, cx| t.set_skill_always(name.to_owned(), on, cx));
    }
    Ok(json!("ok"))
}
