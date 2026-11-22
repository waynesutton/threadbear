// Add from Linear: pick Linear issues and they come in as tasks
// (linear.search and linear.import in runner/src/linear-tasks.ts). A search
// box (a pasted key or link finds that one issue), filters for team,
// project, assigned to me and open only, and issues already in Tasks shown
// but not pickable. The footer picks the Threadbear project they go in,
// whether they keep in sync, and whether sub-issues come as sub-tasks.
//
// Like the send sheet, it's handed what it needs and only calls back into
// the Tasks page from its own handlers.
use crate::linear_send::linear_mark;
use crate::tasks_page::TasksPage;
use crate::tips::Tip as _;
use crate::{
    call, icons,
    style::{self, px},
};
use gpui_kit::base::TestTextExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::sync::Arc;
use threadbear_client::runner::Client;

/// linear.import takes up to 25 issues at a time.
const MAX_PICKS: usize = 25;

pub(crate) fn open(
    client: Arc<Client>,
    page: WeakEntity<TasksPage>,
    projects: Vec<(String, Value)>,
    project: Value,
    window: &mut Window,
    cx: &mut App,
) {
    let view = cx.new(|cx| LinearImport::new(client, page, projects, project, window, cx));
    let focus = view.read(cx).query.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .w(px(640.))
            .close_button(false)
            .child(view.clone())
            .footer(div())
    });
    focus.update(cx, |f, cx| f.focus(window, cx));
    crate::web_panels::keyboard_to_app(window);
}

pub(crate) struct LinearImport {
    client: Arc<Client>,
    page: WeakEntity<TasksPage>,
    query: Entity<InputState>,
    teams: Vec<(String, String)>,
    team: Option<String>,
    projects: Vec<(String, String)>,
    project: Option<String>,
    project_asks: u64,
    assigned: bool,
    open_only: bool,
    issues: Vec<Value>,
    cursor: Option<String>,
    searching: bool,
    /// The newest search; an older answer is dropped.
    asks: u64,
    picked: Vec<String>,
    tb_projects: Vec<(String, Value)>,
    tb_project: Value,
    sync: bool,
    sub_issues: bool,
    adding: bool,
    error: Option<String>,
    debounce: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl LinearImport {
    fn new(
        client: Arc<Client>,
        page: WeakEntity<TasksPage>,
        tb_projects: Vec<(String, Value)>,
        tb_project: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search, or paste an issue key or link")
        });
        let sub = cx.subscribe(&query, |this, _, ev: &InputEvent, cx| match ev {
            InputEvent::Change => this.search_soon(cx),
            InputEvent::PressEnter { .. } => {
                this.debounce = None;
                this.search(false, cx);
            }
            _ => {}
        });
        let mut this = Self {
            client,
            page,
            query,
            teams: vec![],
            team: None,
            projects: vec![],
            project: None,
            project_asks: 0,
            assigned: true,
            open_only: true,
            issues: vec![],
            cursor: None,
            searching: false,
            asks: 0,
            picked: vec![],
            tb_projects,
            tb_project,
            sync: true,
            sub_issues: false,
            adding: false,
            error: None,
            debounce: None,
            _subs: vec![sub],
        };
        this.load_teams(cx);
        this.search(false, cx);
        this
    }

    fn load_teams(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "linear.options", json!({})).await;
            this.update(cx, |this, cx| {
                if let Ok(v) = result {
                    this.teams = v["teams"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|t| {
                            Some((
                                t["id"].as_str()?.to_string(),
                                t["displayName"]
                                    .as_str()
                                    .or(t["name"].as_str())
                                    .unwrap_or("Team")
                                    .to_string(),
                            ))
                        })
                        .collect();
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn set_team(&mut self, team: Option<String>, cx: &mut Context<Self>) {
        if self.team == team {
            return;
        }
        self.team = team.clone();
        self.project = None;
        self.projects = vec![];
        self.project_asks += 1;
        self.search(false, cx);
        let Some(team) = team else { return };
        let (client, ask) = (self.client.clone(), self.project_asks);
        cx.spawn(async move |this, cx| {
            let result = call(client, "linear.projects", json!({"teamId": team})).await;
            this.update(cx, |this, cx| {
                if this.project_asks != ask {
                    return;
                }
                if let Ok(v) = result {
                    this.projects = v["projects"]
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
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Typing searches once it pauses.
    fn search_soon(&mut self, cx: &mut Context<Self>) {
        self.debounce = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(350))
                .await;
            this.update(cx, |this, cx| this.search(false, cx)).ok();
        }));
    }

    /// linear.search with the filters; `more` asks for the next page.
    fn search(&mut self, more: bool, cx: &mut Context<Self>) {
        let mut args = json!({"assignedToMe": self.assigned, "openOnly": self.open_only});
        let query = self.query.read(cx).value().trim().to_string();
        if !query.is_empty() {
            args["query"] = json!(query);
        }
        if let Some(team) = &self.team {
            args["teamId"] = json!(team);
        }
        if let Some(project) = &self.project {
            args["projectId"] = json!(project);
        }
        if more {
            let Some(cursor) = self.cursor.clone() else {
                return;
            };
            args["cursor"] = json!(cursor);
        }
        self.asks += 1;
        let ask = self.asks;
        self.searching = true;
        self.error = None;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "linear.search", args).await;
            this.update(cx, |this, cx| {
                if this.asks != ask {
                    return;
                }
                this.searching = false;
                match result {
                    Ok(v) => {
                        let found = v["issues"].as_array().cloned().unwrap_or_default();
                        if more {
                            this.issues.extend(found);
                        } else {
                            this.issues = found;
                        }
                        this.cursor = v["cursor"].as_str().map(String::from);
                    }
                    Err(e) => this.error = Some(format!("Couldn't search Linear: {}", e.message)),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(i) = self.picked.iter().position(|k| k == key) {
            self.picked.remove(i);
        } else if self.picked.len() < MAX_PICKS {
            self.picked.push(key.to_string());
        }
        cx.notify();
    }

    fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.picked.is_empty() || self.adding {
            return;
        }
        let project = if self.tb_project.is_object() {
            self.tb_project.clone()
        } else {
            Value::Null
        };
        let args = json!({"issues": self.picked, "project": project, "sync": self.sync, "includeSubIssues": self.sub_issues});
        self.adding = true;
        self.error = None;
        cx.notify();
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, "linear.import", args).await;
            this.update_in(cx, |this, window, cx| {
                this.adding = false;
                match result {
                    Ok(v) => {
                        let created = v["created"].as_array().map_or(0, |a| a.len());
                        let skipped = v["skipped"].as_array().map_or(0, |a| a.len());
                        let note = result_note(created, skipped);
                        this.page.update(cx, |p, cx| p.linear_note(note, cx)).ok();
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

    fn filter_button(
        &self,
        id: &'static str,
        label: String,
        tip: &'static str,
        choices: Vec<(Option<String>, String)>,
        chosen: Option<String>,
        team: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let sheet = cx.entity().downgrade();
        Button::new(id)
            .label(label)
            .small()
            .outline()
            .tooltip(tip)
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for (value, name) in &choices {
                    let (s, value) = (sheet.clone(), value.clone());
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(chosen == value)
                            .on_click(move |_, _, cx| {
                                s.update(cx, |s, cx| {
                                    if team {
                                        s.set_team(value.clone(), cx);
                                    } else if s.project != value {
                                        s.project = value.clone();
                                        s.search(false, cx);
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

    fn issue_row(&self, issue: &Value, cx: &mut Context<Self>) -> Option<AnyElement> {
        let key = issue["identifier"].as_str()?.to_string();
        let title = issue["title"].as_str().unwrap_or("").to_string();
        let status = issue["status"].as_str().unwrap_or("").to_string();
        let project = issue["project"].as_str().unwrap_or("").to_string();
        let in_tasks = issue["inTasks"].is_object();
        let picked = self.picked.contains(&key);
        let full = !picked && self.picked.len() >= MAX_PICKS;
        let tertiary = style::tertiary_label(cx);
        let hover = style::hover_fill(cx);
        let k = key.clone();
        Some(
            h_flex()
                .id(SharedString::from(format!("linear-issue-{key}")))
                .gap(px(10.))
                .px(px(6.))
                .py(px(6.))
                .border_b_1()
                .border_color(style::hairline(cx))
                .when(!in_tasks && !full, |d| {
                    d.cursor_pointer()
                        .hover(move |s| s.bg(hover))
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle(&k, cx)))
                })
                .when(in_tasks, |d| d.opacity(0.6))
                .child(
                    Checkbox::new(SharedString::from(format!("linear-pick-{key}")))
                        .checked(picked || in_tasks)
                        .disabled(in_tasks || full)
                        .accessibility_label(format!("Add {key}"))
                        // The row takes the click, so the box and the words do the same.
                        .tooltip(if in_tasks {
                            "Already in Tasks"
                        } else if full {
                            "Up to 25 at a time"
                        } else if picked {
                            "Leave it out"
                        } else {
                            "Add it"
                        }),
                )
                .child(
                    div()
                        .w(px(72.))
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(tertiary)
                        .child(key.clone().test_text(format!("Issue {key}"))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(14.))
                        .child(title.clone().test_text(title)),
                )
                .when(!project.is_empty(), |d| {
                    d.child(
                        div()
                            .max_w(px(120.))
                            .truncate()
                            .text_size(px(12.))
                            .text_color(tertiary)
                            .child(project),
                    )
                })
                .child(
                    div()
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(style::secondary_label(cx))
                        .child(if in_tasks {
                            "In Tasks".to_string()
                        } else {
                            status
                        }),
                )
                .into_any_element(),
        )
    }
}

/// "Added 3, 1 was already in Tasks".
fn result_note(created: usize, skipped: usize) -> String {
    let already = |n: usize| {
        if n == 1 {
            "1 was already in Tasks".to_string()
        } else {
            format!("{n} were already in Tasks")
        }
    };
    match (created, skipped) {
        (0, 0) => "Nothing was added".into(),
        (0, s) => format!("Nothing added: {}", already(s)),
        (c, 0) => format!("Added {c} from Linear"),
        (c, s) => format!("Added {c}, {}", already(s)),
    }
}

impl Render for LinearImport {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (secondary, tertiary) = (style::secondary_label(cx), style::tertiary_label(cx));
        let team_name = self
            .team
            .as_ref()
            .and_then(|id| self.teams.iter().find(|(t, _)| t == id))
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| "All teams".into());
        let mut team_choices = vec![(None, "All teams".to_string())];
        team_choices.extend(
            self.teams
                .iter()
                .map(|(id, n)| (Some(id.clone()), n.clone())),
        );
        let team = self.filter_button(
            "linear-import-team",
            team_name,
            "Only this team's issues",
            team_choices,
            self.team.clone(),
            true,
            cx,
        );
        let project_name = self
            .project
            .as_ref()
            .and_then(|id| self.projects.iter().find(|(p, _)| p == id))
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| "All projects".into());
        let mut project_choices = vec![(None, "All projects".to_string())];
        project_choices.extend(
            self.projects
                .iter()
                .map(|(id, n)| (Some(id.clone()), n.clone())),
        );
        let project =
            div()
                .when(self.team.is_none(), |d| d.opacity(0.5))
                .child(if self.team.is_some() {
                    self.filter_button(
                        "linear-import-project",
                        project_name,
                        "Only this project's issues",
                        project_choices,
                        self.project.clone(),
                        false,
                        cx,
                    )
                } else {
                    Button::new("linear-import-project")
                        .label("All projects")
                        .small()
                        .outline()
                        .disabled(true)
                        .tooltip("Choose a team to filter by project")
                        .into_any_element()
                });

        let mut list = v_flex()
            .id("linear-import-list")
            .h(px(320.))
            .overflow_y_scroll();
        let rows: Vec<AnyElement> = self
            .issues
            .clone()
            .iter()
            .filter_map(|i| self.issue_row(i, cx))
            .collect();
        if rows.is_empty() {
            let empty = if self.searching {
                "Searching Linear…"
            } else if self.error.is_some() {
                ""
            } else {
                "No issues match"
            };
            list = list.child(
                div()
                    .p(px(16.))
                    .text_size(px(13.))
                    .text_color(tertiary)
                    .child(empty.test_text(empty)),
            );
        }
        list = list.children(rows);
        if self.cursor.is_some() {
            list = list.child(
                div().p(px(6.)).child(
                    Button::new("linear-import-more")
                        .label("Load more")
                        .small()
                        .ghost()
                        .loading(self.searching)
                        .disabled(self.searching)
                        .tooltip("Show more issues")
                        .on_click(cx.listener(|this, _, _, cx| this.search(true, cx))),
                ),
            );
        }

        let tb_name = self
            .tb_projects
            .iter()
            .find(|(_, v)| *v == self.tb_project)
            .map(|(n, _)| n.clone())
            .unwrap_or_else(|| "No project".into());
        let (sheet, choices, chosen) = (
            cx.entity().downgrade(),
            self.tb_projects.clone(),
            self.tb_project.clone(),
        );
        let tb_button = Button::new("linear-import-tb-project")
            .label(tb_name)
            .icon(icons::symbol("square.stack"))
            .small()
            .outline()
            .tooltip("The Threadbear project the tasks go in")
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for (label, value) in &choices {
                    let (s, value) = (sheet.clone(), value.clone());
                    menu = menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(chosen == value)
                            .on_click(move |_, _, cx| {
                                s.update(cx, |s, cx| {
                                    s.tb_project = value.clone();
                                    cx.notify();
                                })
                                .ok();
                            }),
                    );
                }
                menu
            });
        let count = self.picked.len();
        v_flex()
            .id("linear-import")
            .role(Role::Dialog)
            .aria_label("Add from Linear")
            .gap(px(10.))
            .text_color(style::label_text(cx))
            .child(
                h_flex()
                    .gap(px(8.))
                    .child(linear_mark(16., style::label_text(cx)))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Add from Linear"),
                    ),
            )
            .child(
                div()
                    .id("linear-import-search")
                    .tip("Search titles, or paste an issue key like ENG-123 or its link")
                    .child(Input::new(&self.query).small()),
            )
            .child(
                h_flex()
                    .gap(px(8.))
                    .flex_wrap()
                    .items_center()
                    .child(team)
                    .child(project)
                    .child(
                        Checkbox::new("linear-import-mine")
                            .label("Assigned to me")
                            .checked(self.assigned)
                            .tooltip("Only issues assigned to you")
                            .on_click(cx.listener(|this, on: &bool, _, cx| {
                                this.assigned = *on;
                                this.search(false, cx);
                            })),
                    )
                    .child(
                        Checkbox::new("linear-import-open")
                            .label("Open only")
                            .checked(self.open_only)
                            .tooltip("Leave out done and canceled issues")
                            .on_click(cx.listener(|this, on: &bool, _, cx| {
                                this.open_only = *on;
                                this.search(false, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(style::card_border(cx))
                    .child(list),
            )
            .when_some(self.error.clone(), |d, e| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .text_color(style::failed_text(cx))
                        .child(e.clone().test_text(e)),
                )
            })
            .child(
                h_flex()
                    .gap(px(10.))
                    .flex_wrap()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(secondary)
                            .child("Add to"),
                    )
                    .child(tb_button)
                    .child(
                        Checkbox::new("linear-import-sync")
                            .label("Keep in sync")
                            .checked(self.sync)
                            .tooltip("Their status and comments follow Linear")
                            .on_click(cx.listener(|this, on: &bool, _, cx| {
                                this.sync = *on;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("linear-import-subs")
                            .label("Include sub-issues as sub-tasks")
                            .checked(self.sub_issues)
                            .tooltip("Each issue's sub-issues come in under it")
                            .on_click(cx.listener(|this, on: &bool, _, cx| {
                                this.sub_issues = *on;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(8.))
                    .pt(px(4.))
                    .child(div().flex_1())
                    .child(
                        Button::new("linear-import-cancel")
                            .label("Cancel")
                            .small()
                            .outline()
                            .tooltip("Close without adding (Esc)")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                    )
                    .child(
                        Button::new("linear-import-add")
                            .label(if count == 0 {
                                "Add".to_string()
                            } else {
                                format!("Add {count}")
                            })
                            .small()
                            .primary()
                            .loading(self.adding)
                            .disabled(count == 0 || self.adding)
                            .tooltip(if count == 0 {
                                "Pick issues to add"
                            } else {
                                "Add them as tasks"
                            })
                            .on_click(cx.listener(|this, _, window, cx| this.add(window, cx))),
                    ),
            )
    }
}
