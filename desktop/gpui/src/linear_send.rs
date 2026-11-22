// Send to Linear: the sheet that turns one task into a Linear issue
// (linear.send in runner/src/linear-tasks.ts). It says which workspace,
// starts on the team and project last used for the task's Threadbear
// project, shows exactly what goes (linear.preview: the title, the
// description, the sub-tasks as a checklist, the pictures to upload), and
// Create hands it to the runner, which makes the issue in the background:
// the row and the task's page say Sending… and then show the issue's key.
//
// It's handed everything it needs from the Tasks page, and only calls back
// into the page from its own handlers, never while the page is updating.
use crate::tasks_page::TasksPage;
use crate::tips::Tip as _;
use crate::{
    call, icons,
    style::{self, px},
};
use gpui_kit::base::TestTextExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Icon, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use threadbear_client::runner::Client;

/// Linear's mark, drawn as a mask so it takes the text colour like the
/// app's other symbols.
const LINEAR_MARK: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path fill="black" d="M1.2 9.6l5.2 5.2A7 7 0 0 1 1.2 9.6zM1 7.7l7.3 7.3c.6 0 1.2-.2 1.7-.4L1.4 6c-.2.5-.3 1.1-.4 1.7zM2 4.7l9.3 9.3c.4-.3.8-.6 1.2-.9L2.9 3.5c-.3.4-.6.8-.9 1.2zM4.1 2.5A7 7 0 1 1 13.5 11.9z"/></svg>"#;

pub(crate) fn linear_mark(size: f32, color: Hsla) -> Icon {
    Icon::default()
        .data(LINEAR_MARK)
        .with_size(px(size))
        .text_color(color)
}

/// What the Tasks page hands the sheet.
pub(crate) struct SendRequest {
    pub client: Arc<Client>,
    pub page: WeakEntity<TasksPage>,
    pub task_id: String,
    pub title: String,
    /// The task's Threadbear project, as prefs.linear.defaults keys it ("runnerId/id" or "none").
    pub project_key: String,
    pub defaults: Vec<Value>,
    /// Pictures the task's page already has, by imageUrl.
    pub images: HashMap<String, Arc<Image>>,
}

pub(crate) fn open(request: SendRequest, window: &mut Window, cx: &mut App) {
    let view = cx.new(|cx| LinearSend::new(request, window, cx));
    let focus = view.read(cx).title.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .w(px(540.))
            .close_button(false)
            .child(view.clone())
            .footer(div())
    });
    focus.update(cx, |f, cx| f.focus(window, cx));
    crate::web_panels::keyboard_to_app(window);
}

#[derive(Clone)]
struct Team {
    id: String,
    name: String,
}

pub(crate) struct LinearSend {
    client: Arc<Client>,
    page: WeakEntity<TasksPage>,
    task_id: String,
    first_title: String,
    project_key: String,
    defaults: Vec<Value>,
    workspace: Option<String>,
    teams: Option<Vec<Team>>,
    team: Option<String>,
    /// The chosen team's open projects (id, name); None while they load.
    projects: Option<Vec<(String, String)>>,
    project: Option<String>,
    project_asks: u64,
    title: Entity<InputState>,
    preview: Option<Value>,
    images: HashMap<String, Option<Arc<Image>>>,
    picked: HashSet<String>,
    sync: bool,
    sending: bool,
    error: Option<String>,
}

impl LinearSend {
    fn new(r: SendRequest, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Issue title")
                .default_value(r.title.clone())
        });
        let mut this = Self {
            client: r.client,
            page: r.page,
            task_id: r.task_id,
            first_title: r.title,
            project_key: r.project_key,
            defaults: r.defaults,
            workspace: None,
            teams: None,
            team: None,
            projects: None,
            project: None,
            project_asks: 0,
            title,
            preview: None,
            images: r.images.into_iter().map(|(k, v)| (k, Some(v))).collect(),
            picked: HashSet::new(),
            sync: true,
            sending: false,
            error: None,
        };
        this.load(window, cx);
        this
    }

    /// The remembered team and project for this task's Threadbear project.
    fn default_pick(&self) -> Option<(String, Option<String>)> {
        let d = self
            .defaults
            .iter()
            .find(|d| d["projectKey"] == self.project_key.as_str())?;
        Some((
            d["teamId"].as_str()?.to_string(),
            d["projectId"].as_str().map(String::from),
        ))
    }

    fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, "linear.options", json!({})).await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(v) => {
                        let name = v["workspace"]["name"]
                            .as_str()
                            .unwrap_or("Linear")
                            .to_string();
                        this.workspace = Some(name);
                        let teams: Vec<Team> = v["teams"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|t| {
                                let id = t["id"].as_str()?.to_string();
                                let name = t["displayName"]
                                    .as_str()
                                    .or(t["name"].as_str())
                                    .unwrap_or("Team")
                                    .to_string();
                                Some(Team { id, name })
                            })
                            .collect();
                        let remembered = this
                            .default_pick()
                            .filter(|(team, _)| teams.iter().any(|t| &t.id == team));
                        let (team, project) = match remembered {
                            Some((team, project)) => (Some(team), project),
                            None => (teams.first().map(|t| t.id.clone()), None),
                        };
                        if teams.is_empty() {
                            this.error =
                                Some("This Linear workspace has no teams you can send to.".into());
                        }
                        this.teams = Some(teams);
                        if let Some(team) = team {
                            this.pick_team(team, project, cx);
                        }
                    }
                    Err(e) => {
                        this.teams = Some(vec![]);
                        this.error =
                            Some(format!("Couldn't read your Linear teams: {}", e.message));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        let (client, id) = (self.client.clone(), self.task_id.clone());
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, "linear.preview", json!({"taskId": id})).await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(v) => {
                        // The task's title as Linear will get it, unless it's been typed over.
                        if let Some(t) = v["title"].as_str() {
                            if this.title.read(cx).value().as_ref() == this.first_title.as_str()
                                && t != this.first_title
                            {
                                let t = t.to_string();
                                this.title.update(cx, |f, cx| f.set_value(t, window, cx));
                            }
                        }
                        let pictures = v["pictures"].as_array().cloned().unwrap_or_default();
                        this.picked = pictures
                            .iter()
                            .filter_map(|p| p["id"].as_str().map(String::from))
                            .collect();
                        for url in pictures
                            .iter()
                            .filter_map(|p| p["imageUrl"].as_str().map(String::from))
                        {
                            this.load_image(url, cx);
                        }
                        this.preview = Some(v);
                    }
                    Err(e) => this.error = Some(format!("Couldn't read the task: {}", e.message)),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn load_image(&mut self, url: String, cx: &mut Context<Self>) {
        if self.images.contains_key(&url) {
            return;
        }
        self.images.insert(url.clone(), None);
        let fetch = cx
            .background_executor()
            .spawn(crate::tasks_page::fetch_picture(url.clone()));
        cx.spawn(async move |this, cx| {
            let result = fetch.await;
            this.update(cx, |this, cx| {
                if let Ok(image) = result {
                    this.images.insert(url, Some(image));
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// A team, and its open projects: the project kept only when it's one of them.
    fn pick_team(&mut self, team: String, project: Option<String>, cx: &mut Context<Self>) {
        self.team = Some(team.clone());
        self.projects = None;
        self.project = None;
        self.project_asks += 1;
        let ask = self.project_asks;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "linear.projects", json!({"teamId": team})).await;
            this.update(cx, |this, cx| {
                if this.project_asks != ask {
                    return;
                }
                match result {
                    Ok(v) => {
                        let list: Vec<(String, String)> = v["projects"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|p| {
                                Some((
                                    p["id"].as_str()?.to_string(),
                                    p["name"].as_str().unwrap_or("Project").to_string(),
                                ))
                            })
                            .collect();
                        this.project = project.filter(|p| list.iter().any(|(id, _)| id == p));
                        this.projects = Some(list);
                    }
                    Err(e) => {
                        this.projects = Some(vec![]);
                        this.error =
                            Some(format!("Couldn't read the team's projects: {}", e.message));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn loading(&self) -> bool {
        self.teams.is_none()
            || self.preview.is_none()
            || (self.team.is_some() && self.projects.is_none())
    }
    fn can_create(&self, cx: &App) -> bool {
        self.team.is_some()
            && !self.loading()
            && !self.sending
            && !self.title.read(cx).value().trim().is_empty()
    }

    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_create(cx) {
            return;
        }
        let Some(team) = self.team.clone() else {
            return;
        };
        let title = self.title.read(cx).value().trim().to_string();
        let pictures = self
            .preview
            .as_ref()
            .and_then(|p| p["pictures"].as_array().cloned())
            .unwrap_or_default();
        let image_ids: Vec<String> = pictures
            .iter()
            .filter_map(|p| p["id"].as_str().map(String::from))
            .filter(|id| self.picked.contains(id))
            .collect();
        let mut args = json!({"taskId": self.task_id, "teamId": team, "title": title, "imageIds": image_ids, "sync": self.sync});
        if let Some(project) = &self.project {
            args["projectId"] = json!(project);
        }
        self.sending = true;
        self.error = None;
        cx.notify();
        let client = self.client.clone();
        let team_name = self
            .teams
            .as_ref()
            .and_then(|t| t.iter().find(|t| t.id == team))
            .map(|t| t.name.clone())
            .unwrap_or_default();
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, "linear.send", args).await;
            this.update_in(cx, |this, window, cx| {
                this.sending = false;
                match result {
                    Ok(_) => {
                        let (page, key, task, project, sync) = (
                            this.page.clone(),
                            this.project_key.clone(),
                            this.task_id.clone(),
                            this.project.clone(),
                            this.sync,
                        );
                        page.update(cx, |p, cx| {
                            p.save_linear_default(&key, &team, project.as_deref(), cx);
                            p.linear_sent(&task, team_name, sync, cx);
                        })
                        .ok();
                        window.close_dialog(cx);
                    }
                    Err(e) => this.error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn team_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let teams = self.teams.clone().unwrap_or_default();
        let current = self
            .team
            .as_ref()
            .and_then(|id| teams.iter().find(|t| &t.id == id))
            .map(|t| t.name.clone());
        let sheet = cx.entity().downgrade();
        let chosen = self.team.clone();
        Button::new("linear-send-team")
            .label(current.unwrap_or_else(|| {
                if self.teams.is_none() {
                    "Loading…".into()
                } else {
                    "Choose a team".into()
                }
            }))
            .small()
            .outline()
            .disabled(teams.is_empty() || self.sending)
            .tooltip("The Linear team the issue goes to")
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for team in &teams {
                    let (s, id) = (sheet.clone(), team.id.clone());
                    menu = menu.item(
                        PopupMenuItem::new(team.name.clone())
                            .checked(chosen.as_deref() == Some(team.id.as_str()))
                            .on_click(move |_, _, cx| {
                                s.update(cx, |s, cx| {
                                    if s.team.as_deref() != Some(id.as_str()) {
                                        s.pick_team(id.clone(), None, cx)
                                    }
                                })
                                .ok();
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }

    fn project_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let projects = self.projects.clone().unwrap_or_default();
        let current = match (&self.projects, &self.project) {
            (None, _) if self.team.is_some() => "Loading…".to_string(),
            (_, Some(id)) => projects
                .iter()
                .find(|(p, _)| p == id)
                .map(|(_, n)| n.clone())
                .unwrap_or_else(|| "No project".into()),
            _ => "No project".into(),
        };
        let sheet = cx.entity().downgrade();
        let chosen = self.project.clone();
        Button::new("linear-send-project")
            .label(current)
            .small()
            .outline()
            .disabled(self.projects.is_none() || self.sending)
            .tooltip("An open project in that team, or none")
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let s = sheet.clone();
                let mut menu = menu.item(
                    PopupMenuItem::new("No project")
                        .checked(chosen.is_none())
                        .on_click(move |_, _, cx| {
                            s.update(cx, |s, cx| {
                                s.project = None;
                                cx.notify();
                            })
                            .ok();
                        }),
                );
                for (id, name) in &projects {
                    let (s, id) = (sheet.clone(), id.clone());
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(chosen.as_deref() == Some(id.as_str()))
                            .on_click(move |_, _, cx| {
                                s.update(cx, |s, cx| {
                                    s.project = Some(id.clone());
                                    cx.notify();
                                })
                                .ok();
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }
}

impl Render for LinearSend {
    #[expect(
        clippy::too_many_lines,
        reason = "baselined when the lint came in with Mike's 21 Nov 2026 release; split it when it's next worked on"
    )]
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (secondary, tertiary) = (style::secondary_label(cx), style::tertiary_label(cx));
        let label = |text: &'static str| {
            div()
                .w(px(64.))
                .flex_none()
                .text_size(px(12.))
                .text_color(secondary)
                .child(text)
        };
        let workspace = self
            .workspace
            .clone()
            .map(|w| format!("{w} workspace"))
            .unwrap_or_else(|| "Finding your Linear workspace…".into());
        let preview = self.preview.clone();
        let description = preview
            .as_ref()
            .and_then(|p| p["description"].as_str())
            .unwrap_or("")
            .to_string();
        let subtasks = preview
            .as_ref()
            .and_then(|p| p["subtasks"].as_u64())
            .unwrap_or(0);
        let pictures = preview
            .as_ref()
            .and_then(|p| p["pictures"].as_array().cloned())
            .unwrap_or_default();
        let can_create = self.can_create(cx);

        let mut sheet = v_flex()
            .id("linear-send")
            .role(Role::Dialog)
            .aria_label("Send to Linear")
            .gap(px(12.))
            .text_color(style::label_text(cx))
            .child(
                v_flex()
                    .gap(px(2.))
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(linear_mark(16., style::label_text(cx)))
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Send to Linear"),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(tertiary)
                            .child(workspace.clone().test_text(workspace)),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(10.))
                    .child(label("Team"))
                    .child(self.team_button(cx)),
            )
            .child(
                h_flex()
                    .gap(px(10.))
                    .child(label("Project"))
                    .child(self.project_button(cx)),
            )
            .child(
                h_flex().gap(px(10.)).child(label("Title")).child(
                    div()
                        .id("linear-send-title")
                        .flex_1()
                        .min_w_0()
                        .tip("The issue's title")
                        .child(Input::new(&self.title).small().disabled(self.sending)),
                ),
            );

        // What goes in the description, read-only: exactly what linear.preview says.
        let words = match &preview {
            None => div().text_color(tertiary).child("Loading…"),
            Some(_) if description.trim().is_empty() => {
                div().text_color(tertiary).child("No description")
            }
            Some(_) => div().child(description),
        };
        sheet = sheet.child(
            v_flex()
                .gap(px(4.))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(secondary)
                        .child("Description"),
                )
                .child(
                    div()
                        .id("linear-send-description")
                        .max_h(px(160.))
                        .overflow_y_scroll()
                        .p(px(8.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(style::card_border(cx))
                        .bg(style::card_background(cx))
                        .text_size(px(13.))
                        .child(words),
                ),
        );
        if subtasks > 0 {
            sheet = sheet.child(div().text_size(px(12.)).text_color(tertiary).child(
                if subtasks == 1 {
                    "1 sub-task goes in as a checklist".to_string()
                } else {
                    format!("{subtasks} sub-tasks go in as a checklist")
                },
            ));
        }

        if !pictures.is_empty() {
            let mut row = h_flex().id("linear-send-pictures").gap(px(8.)).flex_wrap();
            for (i, picture) in pictures.iter().enumerate() {
                let Some(id) = picture["id"].as_str().map(String::from) else {
                    continue;
                };
                let title = picture["title"]
                    .as_str()
                    .filter(|t| !t.is_empty())
                    .unwrap_or("Picture")
                    .to_string();
                let image = picture["imageUrl"]
                    .as_str()
                    .and_then(|u| self.images.get(u).cloned().flatten());
                let thumb = match image {
                    Some(image) => img(ImageSource::Image(image))
                        .w(px(96.))
                        .h(px(72.))
                        .rounded(px(6.))
                        .object_fit(ObjectFit::Cover)
                        .into_any_element(),
                    None => div()
                        .w(px(96.))
                        .h(px(72.))
                        .rounded(px(6.))
                        .bg(style::card_background(cx))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            icons::symbol("photo")
                                .with_size(px(16.))
                                .text_color(tertiary),
                        )
                        .into_any_element(),
                };
                let on = self.picked.contains(&id);
                row = row.child(
                    v_flex()
                        .gap(px(4.))
                        .items_start()
                        .child(
                            div()
                                .rounded(px(6.))
                                .border_1()
                                .border_color(style::card_border(cx))
                                .overflow_hidden()
                                .when(!on, |d| d.opacity(0.5))
                                .child(thumb),
                        )
                        .child(
                            Checkbox::new(SharedString::from(format!("linear-send-picture-{i}")))
                                .checked(on)
                                .disabled(self.sending)
                                .label(title.clone())
                                .accessibility_label(format!("Upload {title}"))
                                .tooltip(if on {
                                    "Untick to leave it out"
                                } else {
                                    "Tick to upload it"
                                })
                                .on_click(cx.listener(move |this, on: &bool, _, cx| {
                                    if *on {
                                        this.picked.insert(id.clone());
                                    } else {
                                        this.picked.remove(&id);
                                    }
                                    cx.notify();
                                })),
                        ),
                );
            }
            sheet = sheet.child(
                v_flex()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(secondary)
                            .child("Pictures"),
                    )
                    .child(row)
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(tertiary)
                            .child("Ticked pictures upload to Linear"),
                    ),
            );
        }

        sheet = sheet.child(
            Checkbox::new("linear-send-sync")
                .label("Keep in sync with Linear")
                .checked(self.sync)
                .disabled(self.sending)
                .tooltip("The task's status and comments follow the issue")
                .on_click(cx.listener(|this, on: &bool, _, cx| {
                    this.sync = *on;
                    cx.notify();
                })),
        );
        if let Some(error) = self.error.clone() {
            sheet = sheet.child(
                div()
                    .text_size(px(12.))
                    .text_color(style::failed_text(cx))
                    .child(error.clone().test_text(error)),
            );
        }
        sheet.child(
            h_flex()
                .gap(px(8.))
                .pt(px(4.))
                .child(div().flex_1())
                .child(
                    Button::new("linear-send-cancel")
                        .label("Cancel")
                        .small()
                        .outline()
                        .tooltip("Close without sending (Esc)")
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                )
                .child(
                    Button::new("linear-send-create")
                        .label("Create")
                        .small()
                        .primary()
                        .loading(self.sending)
                        .disabled(!can_create)
                        .tooltip(if self.team.is_none() {
                            "Choose a team first"
                        } else {
                            "Make the issue in Linear"
                        })
                        .on_click(cx.listener(|this, _, window, cx| this.create(window, cx))),
                ),
        )
    }
}
