// Settings › Runners › Cloud runners, and its Add a cloud runner sheet
// (docs/virtual-runners.md, "Setting one up"). Mike's plan has a section per
// virtual runner here; this is it, without the cost line. The runner does
// every step (`cloud.setup`, runner/src/cloud-setup.ts) and this only starts
// them and shows them, asking `cloud.status` again while one is under way, so
// `tb cloud` and the app always say the same. The words are
// threadbear_core::cloud_runners.
use super::{plain_note, section_with_body, SettingsWindow};
use crate::push_button::PushButton;
use crate::{call, icons, style};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{h_flex, v_flex, Sizable as _, StyledExt as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
// The zoom-aware px (style.rs), in place of GPUI's.
use crate::style::px;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use threadbear_client::runner::Client;
use threadbear_core::cloud_runners::{self as rules, Status, Step, StepState};

/// How often the status is asked again while a step is under way.
const POLL: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub(crate) struct CloudDraft {
    /// The last `cloud.status`; None until it answers, or while it can't.
    status: Option<Status>,
    /// Asked since the tab last showed.
    asked: bool,
    /// The runner answered "not on" or "too old": the section says so, or hides.
    unavailable: Option<String>,
    starting: bool,
    poll: Option<Task<()>>,
    /// The sheet while it's open, for the automation's `cloudSetup`.
    sheet: Option<WeakEntity<CloudSheet>>,
}

impl SettingsWindow {
    pub(super) fn cloud_on_show(&mut self, cx: &mut Context<Self>) {
        if !self.cloud.asked {
            self.cloud.asked = true;
            self.load_cloud(cx);
        }
    }

    pub(super) fn cloud_hidden(&mut self) {
        self.cloud.asked = false;
        self.cloud.poll = None;
    }

    fn load_cloud(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.cloud.poll = Some(cx.spawn(async move |this, cx| loop {
            let res = call(client.clone(), "cloud.status", json!({})).await;
            let again = this
                .update(cx, |this, cx| {
                    let again = match res {
                        Ok(v) => {
                            let st = Status::decode(&v);
                            let running = st.running();
                            this.cloud.status = Some(st);
                            this.cloud.unavailable = None;
                            running
                        }
                        // An old runner, or virtual runners off: no section.
                        Err(e) if e.code == -32601 => {
                            this.cloud.unavailable = Some(rules::TOO_OLD.to_owned());
                            false
                        }
                        Err(e) => {
                            this.cloud.unavailable = Some(e.message);
                            false
                        }
                    };
                    cx.notify();
                    again
                })
                .unwrap_or(false);
            if !again {
                break;
            }
            cx.background_executor().timer(POLL).await;
        }));
    }

    /// Check again or Rebuild: the button spins until the runner has it, then
    /// the steps show under the runner as they go.
    fn start_cloud(&mut self, method: &'static str, cx: &mut Context<Self>) {
        if self.cloud.starting || self.cloud.status.as_ref().is_some_and(Status::running) {
            return;
        }
        self.cloud.starting = true;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = call(client, method, json!({})).await;
            this.update(cx, |this, cx| {
                this.cloud.starting = false;
                if let Err(e) = res {
                    this.cloud.unavailable = Some(e.message);
                }
                this.load_cloud(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn open_cloud_sheet(
        &mut self,
        edit: Option<rules::Service>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settings = cx.entity().downgrade();
        let client = self.client.clone();
        let view = cx.new(|cx| CloudSheet::new(client, settings, edit, window, cx));
        self.cloud.sheet = Some(view.downgrade());
        let focus = view.read(cx).token.clone();
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .w(px(560.))
                .close_button(false)
                .overlay_closable(false)
                .child(view.clone())
                .footer(div())
        });
        focus.update(cx, |f, cx| f.focus(window, cx));
        crate::web_panels::keyboard_to_app(window);
    }

    /// The section, or None where cloud runners aren't on for this runner.
    pub(super) fn cloud_section(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.cloud_on_show(cx);
        let me = cx.entity().downgrade();
        let status = self.cloud.status.clone();
        let body = match (&status, &self.cloud.unavailable) {
            (None, _) => return None,
            (Some(st), _) if st.services.is_empty() && !st.running() => {
                let add =
                    PushButton::new("settings-cloud-add", rules::ADD, rules::ADD_TIP).render(cx, {
                        move |_, window, cx| {
                            me.update(cx, |this, cx| this.open_cloud_sheet(None, window, cx))
                                .ok();
                        }
                    });
                let mut rows = vec![super::row_with(
                    h_flex()
                        .min_w_0()
                        .gap(px(8.))
                        .items_center()
                        .child(
                            icons::symbol("cloud")
                                .with_size(px(15.))
                                .text_color(style::secondary_label(cx)),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .text_color(style::secondary_label(cx))
                                .child(rules::EMPTY),
                        )
                        .into_any_element(),
                    add,
                    cx,
                )];
                if let Some(job) = st.job.as_ref().filter(|j| j.error.is_some()) {
                    rows.extend(
                        job.steps
                            .iter()
                            .filter(|s| s.state == StepState::Fail)
                            .map(|s| step_row(s, cx)),
                    );
                }
                super::rows_with_dividers(rows, cx)
            }
            (Some(st), _) => self.cloud_runners_body(st, cx),
        };
        let mut section = section_with_body(
            "settings-cloud",
            "Cloud runners",
            rules::ABOUT,
            rules::ASK,
            body,
            cx,
        );
        if let Some(why) = &self.cloud.unavailable {
            section = v_flex()
                .gap(px(6.))
                .child(section)
                .child(plain_note(why, cx))
                .into_any_element();
        }
        Some(section)
    }

    fn cloud_runners_body(&self, st: &Status, cx: &mut Context<Self>) -> AnyElement {
        let me = cx.entity().downgrade();
        let busy = self.cloud.starting || st.running();
        let mut rows: Vec<AnyElement> = Vec::new();
        for svc in &st.services {
            let mut title = h_flex()
                .min_w_0()
                .flex_wrap()
                .gap(px(8.))
                .items_center()
                .child(
                    icons::symbol("cloud")
                        .with_size(px(15.))
                        .text_color(style::secondary_label(cx)),
                )
                .child(div().font_semibold().child(svc.name.clone()))
                .child(
                    div()
                        .text_color(style::secondary_label(cx))
                        .child(rules::service_line(svc)),
                );
            if let Some((word, state)) = rules::badge(st) {
                title = title.child(div().flex_1()).child(badge(word, state, cx));
            }
            rows.push(super::row_with(
                title.into_any_element(),
                div().into_any_element(),
                cx,
            ));
            rows.push(super::row_with(
                div()
                    .pl(px(23.))
                    .text_color(style::secondary_label(cx))
                    .child("Threads")
                    .into_any_element(),
                div().child(rules::threads_line(svc)).into_any_element(),
                cx,
            ));
        }
        if let Some(job) = &st.job {
            rows.extend(
                job.steps
                    .iter()
                    .filter(|s| s.state != StepState::Skipped)
                    .map(|s| step_row(s, cx)),
            );
        }
        let edit_svc = st.services.first().cloned();
        let buttons = h_flex()
            .gap(px(8.))
            .justify_end()
            .w_full()
            .child(
                PushButton::new("settings-cloud-check", rules::CHECK, rules::CHECK_TIP)
                    .busy(busy)
                    .render(cx, {
                        let me = me.clone();
                        move |_, _, cx| {
                            me.update(cx, |this, cx| this.start_cloud("cloud.check", cx))
                                .ok();
                        }
                    }),
            )
            .child(
                PushButton::new("settings-cloud-rebuild", rules::REBUILD, rules::REBUILD_TIP)
                    .disabled(busy)
                    .render(cx, {
                        let me = me.clone();
                        move |_, _, cx| {
                            me.update(cx, |this, cx| this.start_cloud("cloud.rebuild", cx))
                                .ok();
                        }
                    }),
            )
            .child(
                PushButton::new("settings-cloud-edit", rules::EDIT, rules::EDIT_TIP)
                    .disabled(busy)
                    .render(cx, move |_, window, cx| {
                        let edit = edit_svc.clone();
                        me.update(cx, |this, cx| this.open_cloud_sheet(edit, window, cx))
                            .ok();
                    }),
            );
        rows.push(
            div()
                .px(px(10.))
                .py(px(8.))
                .child(buttons)
                .into_any_element(),
        );
        super::rows_with_dividers(rows, cx)
    }

    #[cfg(feature = "automation")]
    pub(super) fn cloud_dump(&self) -> Value {
        let Some(st) = &self.cloud.status else {
            return json!({ "unavailable": self.cloud.unavailable });
        };
        json!({
            "runners": st.services.iter().map(|s| json!({ "name": s.name, "line": rules::service_line(s), "threads": rules::threads_line(s) })).collect::<Vec<_>>(),
            "steps": st.job.as_ref().map(|j| j.steps.iter().map(rules::step_line).collect::<Vec<_>>()),
            "running": st.running(),
        })
    }
}

fn state_colour(state: StepState, cx: &App) -> Hsla {
    match state {
        StepState::Ok => style::added_text(cx),
        StepState::Warn => style::needs_you_text(cx),
        StepState::Fail => style::failed_text(cx),
        StepState::Doing => style::accent(cx),
        StepState::Todo | StepState::Skipped => style::tertiary_label(cx),
    }
}

fn state_mark(state: StepState, cx: &App) -> AnyElement {
    let colour = state_colour(state, cx);
    let symbol = match state {
        StepState::Ok => "checkmark.circle.fill",
        StepState::Warn => "exclamationmark.triangle.fill",
        StepState::Fail => "xmark.circle.fill",
        StepState::Doing => {
            return crate::spinner::Spinner::new()
                .color(colour)
                .with_size(px(13.))
                .into_any_element()
        }
        StepState::Todo | StepState::Skipped => "circle",
    };
    icons::symbol(symbol)
        .with_size(px(14.))
        .text_color(colour)
        .into_any_element()
}

fn step_row(step: &Step, cx: &App) -> AnyElement {
    // The title, and under it what the step found or why it stopped.
    let words = v_flex()
        .flex_1()
        .gap(px(1.))
        .min_w_0()
        .child(
            div()
                .when(
                    matches!(step.state, StepState::Todo | StepState::Skipped),
                    |d| d.text_color(style::secondary_label(cx)),
                )
                .child(step.title.clone()),
        )
        .when_some(step.detail.clone(), |d, detail| {
            d.child(
                div()
                    .min_w_0()
                    .text_color(style::secondary_label(cx))
                    .text_size(px(12.))
                    .child(detail),
            )
        });
    h_flex()
        .px(px(10.))
        .py(px(7.))
        .gap(px(9.))
        .items_start()
        .text_size(px(13.))
        .line_height(px(17.))
        .child(
            div()
                .flex_none()
                .pt(px(1.))
                .child(state_mark(step.state, cx)),
        )
        .child(words)
        .into_any_element()
}

fn badge(word: &'static str, state: StepState, cx: &App) -> AnyElement {
    let colour = state_colour(state, cx);
    div()
        .flex_none()
        .px(px(7.))
        .rounded(px(9.))
        .text_size(px(11.))
        .line_height(px(17.))
        .text_color(colour)
        .bg(colour.opacity(0.15))
        .child(word)
        .into_any_element()
}

/// Add a cloud runner, or Edit… one: a name, a Fly token (or flyctl's
/// sign-in), the app's name and a box size, then the steps as the runner
/// does them. Nothing is sent until Add; the token field is masked and
/// cleared once it's sent.
pub(crate) struct CloudSheet {
    client: Arc<Client>,
    settings: WeakEntity<SettingsWindow>,
    edit: Option<rules::Service>,
    name: Entity<InputState>,
    pub(crate) token: Entity<InputState>,
    app: Entity<InputState>,
    size: usize,
    flyctl: bool,
    started: bool,
    sending: bool,
    status: Option<Status>,
    error: Option<String>,
    poll: Option<Task<()>>,
}

impl CloudSheet {
    fn new(
        client: Arc<Client>,
        settings: WeakEntity<SettingsWindow>,
        edit: Option<rules::Service>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                edit.as_ref()
                    .map_or_else(|| "Cloud (Fly)".to_owned(), |s| s.name.clone()),
            )
        });
        let token = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder(if edit.is_some() {
                    "Leave empty to keep the saved token"
                } else {
                    "FlyV1 fm2_…"
                })
        });
        // The app's name is the image's: registry.fly.io/<app>:latest.
        let app_name = edit
            .as_ref()
            .and_then(|s| s.image.strip_prefix("registry.fly.io/"))
            .and_then(|s| s.split(':').next())
            .map(str::to_owned);
        let app = cx.new(|cx| {
            let i = InputState::new(window, cx).placeholder("tb-boxes-you");
            match app_name {
                Some(a) => i.default_value(a),
                None => i,
            }
        });
        let size = edit
            .as_ref()
            .and_then(|s| {
                rules::SIZES
                    .iter()
                    .position(|&(c, m)| c == s.cpus && m == s.memory_mb)
            })
            .unwrap_or(rules::DEFAULT_SIZE);
        CloudSheet {
            client,
            settings,
            edit,
            name,
            token,
            app,
            size,
            flyctl: false,
            started: false,
            sending: false,
            status: None,
            error: None,
            poll: None,
        }
    }

    fn running(&self) -> bool {
        self.sending || self.status.as_ref().is_some_and(Status::running)
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running() {
            return;
        }
        let token = self.token.read(cx).value().trim().to_owned();
        let mut params = json!({
            "name": self.name.read(cx).value().trim(),
            "cpus": rules::SIZES[self.size].0,
            "memoryMb": rules::SIZES[self.size].1,
            "useFlyctl": self.flyctl,
        });
        let app = self.app.read(cx).value().trim().to_owned();
        if !app.is_empty() {
            params["app"] = Value::from(app);
        }
        if !token.is_empty() {
            params["token"] = Value::from(token);
        }
        if let Some(svc) = &self.edit {
            params["serviceId"] = Value::from(svc.id.clone());
        }
        // Out of the field as soon as it's sent: the runner has it now.
        self.token.update(cx, |t, cx| t.set_value("", window, cx));
        self.sending = true;
        self.error = None;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = call(client, "cloud.setup", params).await;
            this.update(cx, |this, cx| {
                this.sending = false;
                match res {
                    Ok(_) => {
                        this.started = true;
                        this.follow(cx);
                    }
                    Err(e) if e.code == -32601 => this.error = Some(rules::TOO_OLD.to_owned()),
                    Err(e) => this.error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // The steps as the runner does them, until it's finished; then Settings' section too.
    fn follow(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.poll = Some(cx.spawn(async move |this, cx| loop {
            let res = call(client.clone(), "cloud.status", json!({})).await;
            let again = this
                .update(cx, |this, cx| {
                    let again = match res {
                        Ok(v) => {
                            let st = Status::decode(&v);
                            let running = st.running();
                            this.status = Some(st);
                            running
                        }
                        Err(e) => {
                            this.error = Some(e.message);
                            false
                        }
                    };
                    if !again {
                        this.settings.update(cx, |s, cx| s.load_cloud(cx)).ok();
                    }
                    cx.notify();
                    again
                })
                .unwrap_or(false);
            if !again {
                break;
            }
            cx.background_executor().timer(POLL).await;
        }));
    }
}

impl Render for CloudSheet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let secondary = style::secondary_label(cx);
        let running = self.running();
        let label = |text: &'static str| div().text_size(px(12.)).text_color(secondary).child(text);
        let me = cx.entity().downgrade();

        let mut sizes = h_flex().gap(px(6.));
        for (i, (cpus, mb)) in rules::SIZES.iter().enumerate() {
            let me = me.clone();
            let picked = i == self.size;
            let chip = div()
                .id(SharedString::from(format!("cloud-size-{i}")))
                .px(px(9.))
                .py(px(3.))
                .rounded(px(6.))
                .text_size(px(12.))
                .border_1()
                .border_color(if picked {
                    style::accent(cx)
                } else {
                    style::hairline(cx)
                })
                .when(!running, |d| {
                    d.cursor_pointer().on_click(move |_, _, cx| {
                        me.update(cx, |this, cx| {
                            this.size = i;
                            cx.notify();
                        })
                        .ok();
                    })
                })
                .child(rules::size_label(*cpus, *mb));
            sizes = sizes.child(crate::highlights::tipped(
                chip,
                "The box each thread gets. Fly can pause one of 2 GB or less",
            ));
        }

        let flyctl = PushButton::new("cloud-flyctl", rules::FLYCTL, rules::FLYCTL_TIP)
            .small()
            .disabled(running)
            .render(cx, {
                let me = me.clone();
                // One press sets it up: flyctl makes the token, so there's
                // nothing more to fill in.
                move |_, window, cx| {
                    me.update(cx, |this, cx| {
                        this.flyctl = true;
                        this.send(window, cx);
                    })
                    .ok();
                }
            });

        let mut body = v_flex()
            .gap(px(8.))
            .p(px(4.))
            .text_size(px(13.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_semibold()
                    .child(if self.edit.is_some() {
                        "Edit cloud runner"
                    } else {
                        "Add a cloud runner"
                    }),
            )
            .child(div().text_color(secondary).child(rules::SHEET_ABOUT))
            .when(!self.started, |d| d.child(before_you_start(cx)))
            .child(label("Name"))
            .child(Input::new(&self.name).small().disabled(running))
            .child(label("Fly app for the boxes"))
            .child(Input::new(&self.app).small().disabled(running))
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .child(label("Fly token"))
                    .child(div().flex_1())
                    .child(flyctl),
            )
            .child(
                Input::new(&self.token)
                    .small()
                    .disabled(running || self.flyctl),
            )
            .when(self.flyctl, |d| {
                d.child(div().text_size(px(12.)).text_color(secondary).child(
                    "flyctl on this Mac makes the token. It must be signed in: fly auth login.",
                ))
            })
            .child(label("Box"))
            .child(sizes);

        if self.started {
            if let Some(job) = self.status.as_ref().and_then(|s| s.job.as_ref()) {
                let rows: Vec<AnyElement> = job.steps.iter().map(|s| step_row(s, cx)).collect();
                body = body.child(
                    div()
                        .mt(px(4.))
                        .rounded(px(10.))
                        .bg(style::primary_opacity(cx, 0.03))
                        .child(super::rows_with_dividers(rows, cx)),
                );
            }
        }
        if let Some(e) = &self.error {
            body = body.child(div().text_color(style::failed_text(cx)).child(e.clone()));
        }

        let main = rules::sheet_button(self.status.as_ref(), self.started, self.edit.is_some());
        let cancel = PushButton::new(
            "cloud-cancel",
            "Cancel",
            "Close this. A setup under way carries on in the runner",
        )
        .render(cx, |_, window, cx| {
            window.close_dialog(cx);
        });
        let go = PushButton::new("cloud-add", main, "Set it up with these")
            .busy(running)
            .render(cx, move |_, window, cx| {
                if main == "Done" {
                    window.close_dialog(cx);
                    return;
                }
                me.update(cx, |this, cx| this.send(window, cx)).ok();
            });
        body.child(
            h_flex()
                .mt(px(8.))
                .gap(px(8.))
                .justify_end()
                // Once it's set up there's nothing to cancel: Done alone.
                .when(main != "Done", |d| d.child(cancel))
                .child(go),
        )
    }
}

/// The commands to run first when flyctl isn't on this Mac or signed in yet.
fn before_you_start(cx: &App) -> impl IntoElement {
    let secondary = style::secondary_label(cx);
    let mono = gpui_kit::component::ActiveTheme::theme(cx)
        .mono_font_family
        .clone();
    let mut list = v_flex()
        .gap(px(4.))
        .p(px(8.))
        .rounded(px(8.))
        .bg(style::primary_opacity(cx, 0.04))
        .text_size(px(12.))
        .child(div().text_color(secondary).child(rules::BEFORE));
    for (cmd, what) in rules::BEFORE_STEPS {
        list = list.child(
            h_flex()
                .gap(px(8.))
                .flex_wrap()
                .child(div().font_family(mono.clone()).child(cmd))
                .child(div().text_color(secondary).child(what)),
        );
    }
    list.child(div().text_color(secondary).child(rules::BEFORE_AFTER))
}

/// The automation's `{"cmd":"settings", "cloudSetup": {"token": "…", "app"?: "…"}}`:
/// opens Add a cloud runner…, types the token and app as Mike would, and
/// presses Add (not with `"press": false`, which only opens it).
#[cfg(feature = "automation")]
pub(crate) fn automation(
    app: &Entity<crate::app::Threadbear>,
    cmd: &Value,
    window: &mut Window,
    cx: &mut App,
) -> Result<Option<Value>, String> {
    let Some(setup) = cmd.get("cloudSetup") else {
        return Ok(None);
    };
    let view = app
        .read(cx)
        .settings_window
        .as_ref()
        .map(|(_, v)| v.clone())
        .ok_or("Settings isn't open")?;
    view.update(cx, |this, cx| this.open_cloud_sheet(None, window, cx));
    let sheet = view
        .read(cx)
        .cloud
        .sheet
        .as_ref()
        .and_then(WeakEntity::upgrade)
        .ok_or("the sheet didn't open")?;
    let token = setup["token"].as_str().unwrap_or_default().to_owned();
    let app_name = setup["app"].as_str().map(str::to_owned);
    sheet.update(cx, |s, cx| {
        s.token.update(cx, |t, cx| t.set_value(token, window, cx));
        if let Some(a) = app_name {
            s.app.update(cx, |t, cx| t.set_value(a, window, cx));
        }
        if setup["press"].as_bool() != Some(false) {
            s.send(window, cx);
        }
    });
    Ok(Some(json!("ok")))
}
