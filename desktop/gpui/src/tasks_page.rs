// Cloud-owned Tasks: a quick-add list first, a board second. Row edits show at
// once and roll back with the error; the editor keeps its draft through a
// failed save, so nothing typed is lost to a dropped connection.
use crate::tips::Tip as _;
use crate::a11y::A11yRole as _;
use crate::{app::Threadbear, call, icons, style::{self, px}};
use gpui_kit::base::{TestSupportExt as _, TestTextExt as _};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::HashSet;
use threadbear_core::task_order;

actions!(tasks_page, [CloseTasks]);

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskRow {
    id: String,
    title: String,
    notes: String,
    status: String,
    project: Option<Value>,
    position: f64,
    created_at: f64,
}
#[derive(Clone)]
struct TaskDrag {
    id: String,
    title: String,
}
#[derive(Clone)]
struct StatusDrag(String);
#[derive(Clone)]
struct ProjectDrag(String, String);
struct Ghost(String);
impl Render for Ghost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().px(px(12.)).py(px(8.)).rounded(px(8.)).bg(style::card_background(cx)).border_1().border_color(style::card_border(cx)).shadow_md().text_size(px(13.)).child(self.0.clone())
    }
}

fn status_name(status: &str) -> &'static str {
    match status {
        "backlog" => "Backlog",
        "doing" => "Doing",
        "done" => "Done",
        _ => "Todo",
    }
}
fn status_icon(status: &str, cx: &App) -> Icon {
    let (symbol, color) = match status {
        "backlog" => ("circle.dashed", style::tertiary_label(cx)),
        "doing" => ("circle.lefthalf.filled", Hsla::from(rgb(0xe2a336))),
        "done" => ("checkmark.circle.fill", Hsla::from(rgb(0x4f9d69))),
        _ => ("circle", style::secondary_label(cx)),
    };
    icons::symbol(symbol).with_size(px(14.)).text_color(color)
}
fn project_key(project: &Option<Value>) -> String {
    match project {
        Some(p) if p.is_object() => format!("{}/{}", p["runnerId"].as_str().unwrap_or(""), p["id"].as_str().unwrap_or("")),
        _ => "none".into(),
    }
}
fn strings(value: &Value) -> Vec<String> {
    value.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default()
}

impl Threadbear {
    pub(crate) fn toggle_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_tasks(cx) || !self.experiment_on("tasks") {
            return;
        }
        self.close_settings(cx);
        self.close_notes(cx);
        self.close_skills(cx);
        let app = cx.entity().downgrade();
        let client = self.client.clone();
        let page = self.tasks_draft_page.take().unwrap_or_else(|| cx.new(|cx| TasksPage::new(app, client, window, cx)));
        let focus = page.read(cx).focus.clone();
        window.focus(&focus, cx);
        crate::web_panels::keyboard_to_app(window);
        page.update(cx, |p, cx| {
            p.refresh(cx);
            p.change("tasks.setPreferences", json!({"theme":style::appearance().name()}), cx);
            p.focus_composer(window, cx);
        });
        self.tasks_page = Some(page);
        self.sync_menus(cx);
        cx.notify();
    }
    pub(crate) fn close_tasks(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(page) = self.tasks_page.take() else { return false };
        if page.read(cx).draft_dirty(cx) || page.read(cx).busy {
            self.tasks_draft_page = Some(page);
        }
        let (window, focus) = (self.main_window, self.root_focus.clone());
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    window.focus(&focus, cx);
                    crate::web_panels::keyboard_to_app(window);
                })
                .ok();
        });
        self.sync_menus(cx);
        cx.notify();
        true
    }
    pub(crate) fn load_project_task_count(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.experiment_on("tasks") || self.task_counts.contains_key(id) {
            return;
        }
        let id = id.to_string();
        self.task_counts.insert(id.clone(), None);
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "tasks.projectCount", json!({"project": id})).await;
            this.update(cx, |this, cx| {
                if let Ok(value) = result {
                    this.task_counts.insert(id, Some(value["count"].as_u64().unwrap_or(0) as usize));
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    pub(crate) fn open_project_tasks(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.close_tasks(cx);
        self.toggle_tasks(window, cx);
        if let Some(page) = &self.tasks_page {
            page.update(cx, |p, cx| {
                p.project_filter_id = Some(id);
                cx.notify();
            });
        }
    }
    pub(crate) fn tasks_updated(&mut self, cx: &mut Context<Self>) {
        self.task_counts.clear();
        if let Some(page) = &self.tasks_page {
            page.update(cx, |p, cx| p.refresh(cx));
        }
    }
    pub(crate) fn tasks_have_draft(&self, cx: &App) -> bool {
        self.tasks_page.as_ref().or(self.tasks_draft_page.as_ref()).is_some_and(|p| p.read(cx).busy || p.read(cx).draft_dirty(cx))
    }
    pub(crate) fn tasks_dump(&self, cx: &App) -> Value {
        let page = self.tasks_page.as_ref().map(|p| p.read(cx));
        json!({"open":page.is_some(), "loaded":page.is_some_and(|p| !p.loading && !p.busy && p.prefs["defaultView"].is_string()),
            "view":page.map(|p| p.view_name()),
            "defaultView":page.map(|p| p.prefs["defaultView"].clone()),
            "grouping":page.map(|p| p.prefs["grouping"].clone()),
            "columnOrder":page.map(|p| p.board_order()),
            "listOrder":page.map(|p| p.list_order()),
            "projectOrder":page.map(|p| p.prefs["projectOrder"].clone()),
            "showDone":page.map(|p| p.show_done),
            "editorOpen":page.map(|p| p.editor_open),
            "quickDraft":page.map(|p| p.composer.read(cx).value().to_string()),
            "toast":page.and_then(|p| p.toast.as_ref().map(|t| t.0.clone())),
            "tasks":page.map(|p| p.rows.iter().map(|t| json!({"id":t.id,"title":t.title,"status":t.status,"notes":t.notes,"project":project_key(&t.project)})).collect::<Vec<_>>()),
            "error":page.and_then(|p| p.error.clone())})
    }
    pub(crate) fn render_tasks_page(&self) -> AnyElement {
        match &self.tasks_page {
            Some(page) => div().size_full().child(page.clone()).into_any_element(),
            None => div().into_any_element(),
        }
    }
}

pub(crate) struct TasksPage {
    app: WeakEntity<Threadbear>,
    client: std::sync::Arc<threadbear_client::runner::Client>,
    rows: Vec<TaskRow>,
    prefs: Value,
    // The editor sheet: title, note, status and project of one task.
    title: Entity<InputState>,
    notes: Entity<TextareaState>,
    color: Entity<InputState>,
    composer: Entity<TextareaState>,
    column_input: Entity<InputState>,
    adding_in: Option<(String, String)>,
    projects: Vec<Value>,
    filter: Value,
    photo: Option<String>,
    customize: bool,
    refresh_again: bool,
    draft_id: String,
    project_filter_id: Option<String>,
    focus: FocusHandle,
    editing: Option<String>,
    editing_baseline: Option<TaskRow>,
    status: String,
    project: Option<Value>,
    editor_open: bool,
    view: Option<String>,
    busy: bool,
    loading: bool,
    error: Option<String>,
    show_done: bool,
    collapsed: HashSet<String>,
    // What the last row change was, and the call that puts it back.
    toast: Option<(String, &'static str, Value)>,
    toast_timer: Option<Task<()>>,
    scroll: ScrollHandle,
    background_size: (f32, f32),
    _subscriptions: Vec<Subscription>,
}

impl TasksPage {
    fn new(app: WeakEntity<Threadbear>, client: std::sync::Arc<threadbear_client::runner::Client>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        static BOUND: std::sync::Once = std::sync::Once::new();
        BOUND.call_once(|| cx.bind_keys([KeyBinding::new("escape", CloseTasks, Some("TasksPage"))]));
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        crate::web_panels::keyboard_to_app(window);
        let composer = cx.new(|cx| TextareaState::new(window, cx).placeholder("Add a task. Return to add, Shift-Return for a new line").auto_grow(1, 6).submit_on_enter(true));
        let column_input = cx.new(|cx| InputState::new(window, cx).placeholder("Task title, then Return"));
        let subs = vec![
            cx.subscribe_in(&composer, window, |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { shift: false, .. } = ev {
                    this.submit_composer(window, cx);
                }
            }),
            cx.subscribe_in(&column_input, window, |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.submit_column(window, cx),
                InputEvent::Blur if this.column_input.read(cx).value().trim().is_empty() => {
                    this.adding_in = None;
                    cx.notify();
                }
                _ => {}
            }),
        ];
        Self {
            app,
            client,
            rows: vec![],
            prefs: json!({}),
            title: cx.new(|cx| InputState::new(window, cx).placeholder("Task title")),
            notes: cx.new(|cx| TextareaState::new(window, cx).placeholder("Add a note…").auto_grow(3, 10)),
            color: cx.new(|cx| InputState::new(window, cx).placeholder("#1b1d22")),
            composer,
            column_input,
            adding_in: None,
            projects: vec![],
            filter: json!("all"),
            photo: None,
            customize: false,
            refresh_again: false,
            draft_id: threadbear_core::notes::new_id(),
            project_filter_id: None,
            focus,
            editing: None,
            editing_baseline: None,
            status: "todo".into(),
            project: None,
            editor_open: false,
            view: None,
            busy: false,
            loading: false,
            error: None,
            show_done: false,
            collapsed: HashSet::new(),
            toast: None,
            toast_timer: None,
            scroll: ScrollHandle::new(),
            background_size: (0., 0.),
            _subscriptions: subs,
        }
    }
    fn view_name(&self) -> String {
        self.view.as_deref().or(self.prefs["defaultView"].as_str()).unwrap_or("list").to_string()
    }
    fn board_order(&self) -> Vec<String> {
        task_order::full_order(&strings(&self.prefs["columnOrder"]), &task_order::BOARD)
    }
    fn list_order(&self) -> Vec<String> {
        task_order::full_order(&strings(&self.prefs["listOrder"]), &task_order::LIST)
    }
    fn manual(&self) -> bool {
        self.prefs["sort"].as_str().unwrap_or("manual") == "manual"
    }
    fn by_project(&self) -> bool {
        self.prefs["grouping"] == "project"
    }
    fn draft_dirty(&self, cx: &App) -> bool {
        let title = self.title.read(cx).value().to_string();
        let notes = self.notes.read(cx).value().to_string();
        let typed = !self.composer.read(cx).value().trim().is_empty();
        typed || !self.column_input.read(cx).value().trim().is_empty()
            || match self.editing_baseline.as_ref() {
                Some(row) => title != row.title || notes != row.notes || self.status != row.status || self.project != row.project,
                None => !title.is_empty() || !notes.is_empty(),
            }
    }
    fn focus_composer(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_name() == "list" && !self.editor_open {
            self.composer.update(cx, |c, cx| c.focus(window, cx));
            crate::web_panels::keyboard_to_app(window);
        }
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.loading {
            self.refresh_again = true;
            return;
        }
        let client = self.client.clone();
        self.loading = true;
        cx.spawn(async move |this, cx| {
            let result = async {
                let prefs = call(client.clone(), "tasks.preferences", json!({})).await?;
                let projects = call(client.clone(), "tasks.projects", json!({})).await?;
                let photo = call(client.clone(), "tasks.photo", json!({})).await?;
                let mut rows = vec![];
                let mut after = Value::Null;
                loop {
                    let mut args = json!({"limit": 250});
                    if !after.is_null() {
                        args["after"] = after;
                    }
                    let page = call(client.clone(), "tasks.list", args).await?;
                    if let Some(items) = page["items"].as_array() {
                        rows.extend(items.iter().cloned());
                    }
                    after = page["next"].clone();
                    if after.is_null() {
                        break;
                    }
                }
                Ok::<_, threadbear_client::runner::RpcError>((prefs, rows, projects, photo))
            }
            .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok((prefs, rows, projects, photo)) => {
                        this.prefs = prefs;
                        this.projects = projects.as_array().cloned().unwrap_or_default();
                        this.photo = photo.as_str().map(String::from);
                        if let Some(id) = this.project_filter_id.take() {
                            if let Some(p) = this.projects.iter().find(|p| p["id"] == id) {
                                this.filter = json!({"runnerId":p["runnerId"],"id":p["id"]});
                                this.project = Some(this.filter.clone());
                            }
                        }
                        this.rows = rows.into_iter().filter_map(|r| serde_json::from_value(r).ok()).collect();
                    }
                    Err(e) => this.error = Some(e.message),
                }
                if this.refresh_again {
                    this.refresh_again = false;
                    this.refresh(cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    /// A preference or editor-level change: waits for the runner, then reloads.
    fn change(&mut self, method: &'static str, args: Value, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let client = self.client.clone();
        self.busy = true;
        self.error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = call(client, method, args).await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(_) => this.refresh(cx),
                    Err(e) => this.error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    /// A preference shown at once: the page draws the new value while the runner saves it.
    fn set_pref(&mut self, field: &str, value: Value, cx: &mut Context<Self>) {
        let before = self.prefs.clone();
        self.prefs[field] = value.clone();
        cx.notify();
        let client = self.client.clone();
        let args = json!({ field: value });
        cx.spawn(async move |this, cx| {
            let result = call(client, "tasks.setPreferences", args).await;
            this.update(cx, |this, cx| {
                if let Err(e) = result {
                    this.prefs = before;
                    this.error = Some(e.message);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    /// A row change shown at once and put back, with the error, if the runner refuses it.
    fn patch(&mut self, id: &str, patch: Value, undo: Option<(String, Value)>, cx: &mut Context<Self>) {
        let before = self.rows.clone();
        if let Some(row) = self.rows.iter_mut().find(|r| r.id == id) {
            if let Some(s) = patch["status"].as_str() {
                row.status = s.into();
            }
            if let Some(p) = patch["position"].as_f64() {
                row.position = p;
            }
            if let Some(t) = patch["title"].as_str() {
                row.title = t.into();
            }
            if let Some(n) = patch["notes"].as_str() {
                row.notes = n.into();
            }
            if patch.get("project").is_some() {
                row.project = (!patch["project"].is_null()).then(|| patch["project"].clone());
            }
        }
        self.error = None;
        cx.notify();
        let mut args = patch;
        args["id"] = json!(id);
        self.write("tasks.update", args, before, undo, cx);
    }
    fn write(&mut self, method: &'static str, args: Value, before: Vec<TaskRow>, undo: Option<(String, Value)>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, method, args).await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        if let Some((text, undo)) = undo {
                            this.show_toast(text, if method == "tasks.remove" { "tasks.restore" } else { "tasks.update" }, undo, cx);
                        }
                        this.refresh(cx);
                    }
                    Err(e) => {
                        this.rows = before;
                        this.error = Some(e.message);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn show_toast(&mut self, text: String, method: &'static str, args: Value, cx: &mut Context<Self>) {
        self.toast = Some((text, method, args));
        self.toast_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(std::time::Duration::from_secs(6)).await;
            this.update(cx, |this, cx| {
                this.toast = None;
                cx.notify();
            })
            .ok();
        }));
    }
    fn undo(&mut self, cx: &mut Context<Self>) {
        let Some((_, method, args)) = self.toast.take() else { return };
        if method == "tasks.update" {
            let id = args["id"].as_str().unwrap_or("").to_string();
            let mut patch = args.clone();
            patch.as_object_mut().map(|o| o.remove("id"));
            self.patch(&id, patch, None, cx);
        } else {
            self.change(method, args, cx);
        }
    }
    fn complete(&mut self, task: &TaskRow, cx: &mut Context<Self>) {
        let next = if task.status == "done" { "todo" } else { "done" };
        let undo = (next == "done").then(|| (format!("Done: {}", task.title), json!({"id": task.id, "status": task.status})));
        self.patch(&task.id.clone(), json!({"status": next}), undo, cx);
    }
    fn delete(&mut self, id: &str, cx: &mut Context<Self>) {
        let before = self.rows.clone();
        let title = self.rows.iter().find(|r| r.id == id).map(|r| r.title.clone()).unwrap_or_default();
        self.rows.retain(|r| r.id != id);
        cx.notify();
        self.write("tasks.remove", json!({"id": id}), before, Some((format!("Deleted: {title}"), json!({"id": id}))), cx);
    }
    fn group_project(&self, group: &str) -> Value {
        self.projects.iter().find(|p| format!("{}/{}", p["runnerId"].as_str().unwrap_or(""), p["id"].as_str().unwrap_or("")) == group).map(|p| json!({"runnerId":p["runnerId"],"id":p["id"]})).unwrap_or(Value::Null)
    }
    fn add_task(&mut self, text: &str, status: &str, group: &str, column: bool, window: &mut Window, cx: &mut Context<Self>) {
        let (title, notes) = task_order::split_draft(text);
        if title.is_empty() || self.busy {
            return;
        }
        let project = if group != "all" { self.group_project(group) } else if self.filter.is_object() { self.filter.clone() } else { Value::Null };
        let position = self.rows.iter().filter(|r| r.status == status).map(|r| r.position).min_by(f64::total_cmp).map_or(crate::now_ms() as f64, |p| p - 1000.);
        let id = threadbear_core::notes::new_id();
        self.rows.push(TaskRow { id: id.clone(), title: title.clone(), notes: notes.clone(), status: status.into(), project: (!project.is_null()).then(|| project.clone()), position, created_at: crate::now_ms() as f64 });
        self.busy = true;
        self.error = None;
        cx.notify();
        let args = json!({"id": id, "title": title, "notes": notes, "status": status, "project": project, "position": position});
        let client = self.client.clone();
        // Keep the input until the server accepts it; a rejected quick-add is retryable.
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, "tasks.create", args).await;
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                match result {
                    Ok(_) => {
                        if column {
                            this.column_input.update(cx, |c, cx| c.set_value("", window, cx));
                        } else {
                            this.composer.update(cx, |c, cx| c.set_value("", window, cx));
                        }
                        this.refresh(cx);
                    }
                    Err(e) => {
                        this.rows.retain(|r| r.id != id);
                        this.error = Some(e.message);
                    }
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn submit_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).value().to_string();
        self.add_task(&text, "todo", "all", false, window, cx);
    }
    fn submit_column(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((group, status)) = self.adding_in.clone() else { return };
        let text = self.column_input.read(cx).value().to_string();
        self.add_task(&text, &status, &group, true, window, cx);
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.title.read(cx).value().to_string();
        if title.trim().is_empty() {
            return;
        }
        let mut args = json!({"title": title, "notes": self.notes.read(cx).value().to_string(), "status": self.status, "project": self.project});
        let method = if self.editing_baseline.is_some() { "tasks.update" } else { "tasks.create" };
        if let Some(base) = &self.editing_baseline {
            // Send only what changed here, so another device's edits survive.
            let original = json!({"title":base.title,"notes":base.notes,"status":base.status,"project":base.project});
            args.as_object_mut().unwrap().retain(|key, value| original[key] != *value);
        }
        args["id"] = json!(self.editing.as_ref().unwrap_or(&self.draft_id));
        self.editing = Some(args["id"].as_str().unwrap().to_string());
        if self.busy {
            return;
        }
        let client = self.client.clone();
        self.busy = true;
        self.error = None;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, method, args).await;
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                match result {
                    Ok(_) => {
                        this.close_editor(window, cx);
                        this.refresh(cx);
                    }
                    Err(e) => this.error = Some(e.message),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
    fn close_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor_open = false;
        self.editing = None;
        self.editing_baseline = None;
        self.draft_id = threadbear_core::notes::new_id();
        self.title.update(cx, |f, cx| f.set_value("", window, cx));
        self.notes.update(cx, |f, cx| f.set_value("", window, cx));
        self.focus_composer(window, cx);
    }
    fn open_editor(&mut self, task: &TaskRow, notes_first: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || (self.editor_open && self.draft_dirty_editor(cx)) {
            self.error = Some("Save or discard your draft before opening another task.".into());
            cx.notify();
            return;
        }
        self.editor_open = true;
        self.editing = Some(task.id.clone());
        self.editing_baseline = Some(task.clone());
        self.status = task.status.clone();
        self.project = task.project.clone();
        self.title.update(cx, |f, cx| f.set_value(task.title.clone(), window, cx));
        self.notes.update(cx, |f, cx| f.set_value(task.notes.clone(), window, cx));
        if notes_first {
            self.notes.update(cx, |f, cx| f.focus(window, cx));
            crate::web_panels::keyboard_to_app(window);
        } else {
            self.title.update(cx, |f, cx| f.focus(window, cx));
            crate::web_panels::keyboard_to_app(window);
        }
        cx.notify();
    }
    fn draft_dirty_editor(&self, cx: &App) -> bool {
        let title = self.title.read(cx).value().to_string();
        let notes = self.notes.read(cx).value().to_string();
        match self.editing_baseline.as_ref() {
            Some(row) => title != row.title || notes != row.notes || self.status != row.status || self.project != row.project,
            None => !title.is_empty() || !notes.is_empty(),
        }
    }
    fn project_choices(&self) -> Vec<(String, Value)> {
        let mut out = vec![("No project".into(), Value::Null)];
        out.extend(self.projects.iter().map(|p| (p["title"].as_str().unwrap_or("Project").to_string(), json!({"runnerId":p["runnerId"],"id":p["id"]}))));
        out
    }
    fn project_name(&self, value: &Value) -> String {
        self.projects.iter().find(|p| p["id"] == value["id"] && p["runnerId"] == value["runnerId"]).and_then(|p| p["title"].as_str()).unwrap_or("No project").to_string()
    }
    fn key_name(&self, key: &str) -> String {
        if key == "none" { "No project".into() } else { self.project_name(&self.group_project(key)) }
    }
    fn visible(&self) -> Vec<TaskRow> {
        let mut rows: Vec<TaskRow> = self.rows.iter().filter(|r| self.filter == json!("all") || r.project.clone().unwrap_or(Value::Null) == self.filter).cloned().collect();
        rows.sort_by(|a, b| match self.prefs["sort"].as_str().unwrap_or("manual") {
            "title" => a.title.cmp(&b.title),
            "newest" => b.created_at.total_cmp(&a.created_at),
            "oldest" => a.created_at.total_cmp(&b.created_at),
            _ => a.position.total_cmp(&b.position).then(a.id.cmp(&b.id)),
        });
        rows
    }
    fn groups(&self, visible: &[TaskRow]) -> Vec<String> {
        if !self.by_project() {
            return vec!["all".into()];
        }
        let keys: Vec<String> = visible.iter().map(|r| project_key(&r.project)).collect();
        task_order::group_order(&strings(&self.prefs["projectOrder"]), &keys)
    }
    /// Drops a task before `before`, into `status` and, grouped by project, into `group`.
    fn drop_task(&mut self, drag: &TaskDrag, status: &str, group: &str, before: Option<&str>, cx: &mut Context<Self>) {
        if before == Some(drag.id.as_str()) {
            return;
        }
        let peers: Vec<(String, f64)> = self.visible().into_iter().filter(|r| r.status == status && r.id != drag.id && (group == "all" || project_key(&r.project) == group)).map(|r| (r.id, r.position)).collect();
        let mut patch = json!({"status": status});
        if self.manual() {
            patch["position"] = json!(task_order::position_at(&peers, before, crate::now_ms() as f64));
        }
        if group != "all" {
            patch["project"] = self.group_project(group);
        }
        self.patch(&drag.id.clone(), patch, None, cx);
    }
    fn drop_status(&mut self, drag: &StatusDrag, target: &str, list: bool, cx: &mut Context<Self>) {
        let order = if list { self.list_order() } else { self.board_order() };
        let next = task_order::move_to(&order, &drag.0, target);
        if next != order {
            self.set_pref(if list { "listOrder" } else { "columnOrder" }, json!(next), cx);
        }
    }
    fn drop_project(&mut self, drag: &ProjectDrag, target: &str, cx: &mut Context<Self>) {
        let groups = self.groups(&self.visible());
        let next = task_order::move_to(&groups, &drag.0, target);
        if next != groups {
            self.set_pref("projectOrder", json!(next), cx);
        }
    }

    fn more_menu(&self, task: &TaskRow, cx: &mut Context<Self>) -> AnyElement {
        let page = cx.entity().downgrade();
        let t = task.clone();
        let projects = self.project_choices();
        let statuses = if self.view_name() == "list" { self.list_order() } else { self.board_order() };
        Button::new(SharedString::from(format!("task-more-{}", task.id)))
            .icon(icons::symbol("ellipsis"))
            .ghost()
            .xsmall()
            .tooltip(format!("More for {}", task.title))
            .dropdown_menu(move |menu: PopupMenu, window, cx| {
                let mut menu = menu;
                let (p, row) = (page.clone(), t.clone());
                menu = menu.item(PopupMenuItem::new("Open").icon(icons::symbol("square.and.pencil")).on_click(move |_, window, cx| {
                    p.update(cx, |p, cx| p.open_editor(&row, false, window, cx)).ok();
                }));
                let (p, row) = (page.clone(), t.clone());
                menu = menu.item(PopupMenuItem::new(if t.notes.is_empty() { "Add Note" } else { "Edit Note" }).icon(icons::symbol("note.text")).on_click(move |_, window, cx| {
                    p.update(cx, |p, cx| p.open_editor(&row, true, window, cx)).ok();
                }));
                let (p, row, choices) = (page.clone(), t.clone(), projects.clone());
                menu = menu.submenu_with_icon(Some(icons::symbol("square.stack")), "Move to Project", window, cx, move |sub, _, _| {
                    let mut sub = sub;
                    for (label, value) in &choices {
                        let (p, id, value) = (p.clone(), row.id.clone(), value.clone());
                        let checked = row.project.clone().unwrap_or(Value::Null) == value;
                        sub = sub.item(PopupMenuItem::new(label.clone()).checked(checked).on_click(move |_, _, cx| {
                            p.update(cx, |p, cx| p.patch(&id, json!({"project": value}), None, cx)).ok();
                        }));
                    }
                    sub
                });
                menu = menu.separator();
                for status in &statuses {
                    let (p, id, status) = (page.clone(), t.id.clone(), status.clone());
                    let icon = status_icon(&status, cx);
                    menu = menu.item(PopupMenuItem::new(status_name(&status)).icon(icon).checked(t.status == status).on_click(move |_, _, cx| {
                        p.update(cx, |p, cx| p.patch(&id, json!({"status": status}), None, cx)).ok();
                    }));
                }
                let (p, id) = (page.clone(), t.id.clone());
                menu.separator().item(PopupMenuItem::new("Delete").icon(icons::symbol("trash")).on_click(move |_, _, cx| {
                    p.update(cx, |p, cx| p.delete(&id, cx)).ok();
                }))
            })
            .into_any_element()
    }
    fn chip(&self, task: &TaskRow, cx: &App) -> Option<AnyElement> {
        let project = task.project.as_ref()?;
        if self.by_project() || self.filter != json!("all") {
            return None;
        }
        Some(div().flex_none().max_w(px(160.)).truncate().px(px(8.)).py(px(1.)).rounded_full().border_1().border_color(style::hairline(cx)).text_size(px(12.)).text_color(style::secondary_label(cx)).child(self.project_name(project)).into_any_element())
    }
    fn row(&self, task: &TaskRow, group: &str, cx: &mut Context<Self>) -> AnyElement {
        let id = task.id.clone();
        let next = if task.status == "done" { "not done" } else { "done" };
        let (done, accent, hover) = (task.status == "done", style::accent(cx), style::hover_fill(cx));
        let (drop_status, drop_group, drop_before) = (task.status.clone(), group.to_string(), task.id.clone());
        let check_task = task.clone();
        let edit = task.clone();
        h_flex()
            .id(SharedString::from(format!("task-{id}")))
            .group("task-row")
            .items_start()
            .gap(px(8.))
            .px(px(6.))
            .py(px(7.))
            .border_b_1()
            .border_color(style::hairline(cx))
            .hover(move |s| s.bg(hover))
            .when(self.manual(), |d| d.on_drag(TaskDrag { id: id.clone(), title: task.title.clone() }, |drag, _, _, cx| cx.new(|_| Ghost(drag.title.clone()))))
            .drag_over::<TaskDrag>(move |s, _, _, _| s.border_t_2().border_color(accent))
            .on_drop(cx.listener(move |this, drag: &TaskDrag, _, cx| this.drop_task(drag, &drop_status, &drop_group, Some(&drop_before), cx)))
            .child(
                div().pt(px(2.)).child(
                    Checkbox::new(SharedString::from(format!("check-{id}")))
                        .checked(done)
                        .accessibility_label(format!("Mark {} {}", task.title, next))
                        .tooltip(if done { "Mark not done" } else { "Mark done" })
                        .on_click(cx.listener(move |this, _: &bool, _, cx| this.complete(&check_task, cx))),
                ),
            )
            .child(
                h_flex()
                    .id(SharedString::from(format!("edit-{id}")))
                    .role(Role::Button)
                    .aria_label(SharedString::from(format!("Open {}", task.title)))
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .cursor_pointer()
                    .tip("Open the task")
                    .on_click(cx.listener(move |this, _, window, cx| this.open_editor(&edit, false, window, cx)))
                    .child(div().min_w_0().text_size(px(14.)).when(done, |d| d.line_through().text_color(style::tertiary_label(cx))).child(task.title.clone().test_text(task.title.clone())))
                    .when(!task.notes.is_empty(), |d| d.child(icons::symbol("text.alignleft").with_size(px(12.)).text_color(style::tertiary_label(cx)))),
            )
            .children(self.chip(task, cx))
            .child(div().flex_none().invisible().group_hover("task-row", |s| s.visible()).child(self.more_menu(task, cx)))
            .into_any_element()
    }
    fn card(&self, task: &TaskRow, group: &str, cx: &mut Context<Self>) -> AnyElement {
        let id = task.id.clone();
        let (accent, edge) = (style::accent(cx), style::card_border(cx));
        let (drop_status, drop_group, drop_before) = (task.status.clone(), group.to_string(), task.id.clone());
        let edit = task.clone();
        let done = task.status == "done";
        v_flex()
            .id(SharedString::from(format!("card-{id}")))
            .role(Role::Button)
            .aria_label(SharedString::from(format!("Open {}", task.title)))
            .group("task-card")
            .gap(px(6.))
            .p(px(10.))
            .pl(px(12.))
            .mb(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(edge)
            .bg(cx.theme().background).shadow_sm()
            .cursor_grab()
            .on_drag(TaskDrag { id: id.clone(), title: task.title.clone() }, |drag, _, _, cx| cx.new(|_| Ghost(drag.title.clone())))
            .drag_over::<TaskDrag>(move |s, _, _, _| s.border_t_2().border_color(accent))
            .on_drop(cx.listener(move |this, drag: &TaskDrag, _, cx| this.drop_task(drag, &drop_status, &drop_group, Some(&drop_before), cx)))
            .on_click(cx.listener(move |this, _, window, cx| this.open_editor(&edit, false, window, cx)))
            .tip("Open the task. Drag to move it.")
            .child(
                h_flex()
                    .items_start()
                    .gap(px(4.))
                    .child(div().flex_1().min_w_0().text_size(px(14.)).when(done, |d| d.line_through().text_color(style::tertiary_label(cx))).child(task.title.clone().test_text(task.title.clone())))
                    .child(div().flex_none().invisible().group_hover("task-card", |s| s.visible()).child(self.more_menu(task, cx))),
            )
            .when(!task.notes.is_empty() || self.chip(task, cx).is_some(), |d| {
                d.child(
                    h_flex()
                        .gap(px(6.))
                        .when(!task.notes.is_empty(), |d| d.child(icons::symbol("text.alignleft").with_size(px(12.)).text_color(style::tertiary_label(cx))))
                        .children(self.chip(task, cx)),
                )
            })
            .into_any_element()
    }
    fn fold(&self, key: String, name: String, label: AnyElement, count: usize, cx: &mut Context<Self>) -> AnyElement {
        let folded = self.collapsed.contains(&key);
        let k = key.clone();
        h_flex()
            .id(SharedString::from(format!("fold-{key}")))
            .role(Role::Button)
            .aria_label(SharedString::from(name))
            .aria_expanded(!folded)
            .gap(px(8.))
            .cursor_pointer()
            .tip(if folded { "Show" } else { "Hide" })
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.collapsed.remove(&k) {
                    this.collapsed.insert(k.clone());
                }
                cx.notify();
            }))
            .child(icons::symbol(if folded { "chevron.right" } else { "chevron.down" }).with_size(px(10.)).text_color(style::tertiary_label(cx)))
            .child(label)
            .child(div().text_color(style::tertiary_label(cx)).child(count.to_string()))
            .into_any_element()
    }
    fn list_section(&self, status: &str, group: &str, rows: &[TaskRow], dragging: bool, cx: &mut Context<Self>) -> Option<AnyElement> {
        let items: Vec<&TaskRow> = rows.iter().filter(|r| r.status == status && (group == "all" || project_key(&r.project) == group)).collect();
        if items.is_empty() && !dragging {
            return None;
        }
        let key = format!("status:{group}:{status}");
        let (accent, soft) = (style::accent(cx), style::accent(cx).opacity(0.08));
        let (s1, g1, s2) = (status.to_string(), group.to_string(), status.to_string());
        let label = h_flex().gap(px(8.)).child(status_icon(status, cx)).child(status_name(status)).into_any_element();
        let header = h_flex()
            .id(SharedString::from(format!("section-{key}")))
            .h(px(32.))
            .px(px(8.))
            .rounded(px(6.))
            .bg(style::hover_fill(cx))
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_grab()
            .on_drag(StatusDrag(status.to_string()), |drag, _, _, cx| cx.new(|_| Ghost(status_name(&drag.0).into())))
            .drag_over::<StatusDrag>(move |s, _, _, _| s.border_t_2().border_color(accent))
            .on_drop(cx.listener(move |this, drag: &StatusDrag, _, cx| this.drop_status(drag, &s2, true, cx)))
            .child(self.fold(key.clone(), status_name(status).into(), label, items.len(), cx));
        let mut section = v_flex()
            .id(SharedString::from(key.clone()))
            .mb(px(14.))
            .rounded(px(8.))
            .drag_over::<TaskDrag>(move |s, _, _, _| s.bg(soft))
            .on_drop(cx.listener(move |this, drag: &TaskDrag, _, cx| this.drop_task(drag, &s1, &g1, None, cx)))
            .child(header);
        if !self.collapsed.contains(&key) {
            for task in items {
                section = section.child(self.row(task, group, cx));
            }
        }
        Some(section.into_any_element())
    }
    fn project_header(&self, group: &str, count: usize, cx: &mut Context<Self>) -> AnyElement {
        let accent = style::accent(cx);
        let target = group.to_string();
        let name = self.key_name(group);
        let label = h_flex().gap(px(8.)).child(div().size(px(9.)).rounded(px(3.)).bg(accent.opacity(0.8))).child(name.clone()).into_any_element();
        h_flex()
            .id(SharedString::from(format!("project-{group}")))
            .h(px(32.))
            .mt(px(4.))
            .mb(px(6.))
            .text_size(px(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_grab()
            .tip("Drag to reorder projects")
            .on_drag(ProjectDrag(group.to_string(), name), |drag, _, _, cx| cx.new(|_| Ghost(drag.1.clone())))
            .drag_over::<ProjectDrag>(move |s, _, _, _| s.border_t_2().border_color(accent))
            .on_drop(cx.listener(move |this, drag: &ProjectDrag, _, cx| this.drop_project(drag, &target, cx)))
            .child(self.fold(format!("project:{group}"), self.key_name(group), label, count, cx))
            .into_any_element()
    }
    fn list_view(&self, rows: &[TaskRow], cx: &mut Context<Self>) -> AnyElement {
        let dragging = cx.has_active_drag();
        let mut out = v_flex().w_full().max_w(px(880.)).mx_auto();
        // The quick-add box: type, Return, next.
        out = out.child(
            h_flex()
                .id("tasks-composer")
                .items_start()
                .gap(px(10.))
                .mb(px(18.))
                .p(px(9.))
                .pl(px(12.))
                .rounded(px(10.))
                .border_1()
                .border_color(style::card_border(cx))
                .bg(style::card_background(cx))
                .child(div().mt(px(5.)).size(px(14.)).rounded(px(4.)).border_1().border_dashed().border_color(style::tertiary_label(cx)))
                .child(div().flex_1().child(Textarea::new(&self.composer).appearance(false).disabled(self.busy)))
                .child(Button::new("tasks-add").icon(icons::symbol("arrow.up")).small().primary().loading(self.busy).disabled(self.busy).tooltip("Add task (Return)").on_click(cx.listener(|this, _, window, cx| this.submit_composer(window, cx)))),
        );
        if rows.is_empty() && !self.loading {
            return out.child(v_flex().items_center().gap(px(10.)).py(px(64.)).text_color(style::tertiary_label(cx)).child(status_icon("todo", cx)).child("Nothing here yet. Type a task above and press Return.")).into_any_element();
        }
        let statuses: Vec<String> = self.list_order().into_iter().filter(|s| s != "done" || self.show_done).collect();
        for group in self.groups(rows) {
            let mut block = v_flex().mb(px(18.));
            if group != "all" {
                let count = rows.iter().filter(|r| project_key(&r.project) == group && (self.show_done || r.status != "done")).count();
                block = block.child(self.project_header(&group, count, cx));
                if self.collapsed.contains(&format!("project:{group}")) {
                    out = out.child(block);
                    continue;
                }
            }
            for status in &statuses {
                if let Some(section) = self.list_section(status, &group, rows, dragging, cx) {
                    block = block.child(section);
                }
            }
            out = out.child(block);
        }
        let done = rows.iter().filter(|r| r.status == "done").count();
        if done > 0 && !self.show_done {
            out = out.child(
                h_flex()
                    .id("tasks-show-done")
                    .role(Role::Button)
                    .aria_label(SharedString::from(format!("Show {done} done tasks")))
                    .gap(px(6.))
                    .cursor_pointer()
                    .text_color(style::secondary_label(cx))
                    .tip("Show done tasks")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_done = true;
                        cx.notify();
                    }))
                    .child(status_icon("done", cx))
                    .child(format!("{done} done")),
            );
        }
        out.into_any_element()
    }
    fn board_columns(&self, group: &str, rows: &[TaskRow], cx: &mut Context<Self>) -> AnyElement {
        let mut columns = h_flex().id(SharedString::from(format!("columns-{group}"))).items_start().gap(px(12.)).overflow_x_scroll().pb(px(8.));
        for status in self.board_order() {
            let items: Vec<&TaskRow> = rows.iter().filter(|r| r.status == status && (group == "all" || project_key(&r.project) == group)).collect();
            let (accent, soft) = (style::accent(cx), style::accent(cx).opacity(0.08));
            let (s1, g1, s2, s3, g3) = (status.clone(), group.to_string(), status.clone(), status.clone(), group.to_string());
            let adding = self.adding_in.as_ref() == Some(&(group.to_string(), status.clone()));
            let header = h_flex()
                .id(SharedString::from(format!("column-head-{group}-{status}")))
                .h(px(34.))
                .px(px(6.))
                .gap(px(8.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .cursor_grab()
                .tip("Drag to reorder columns")
                .on_drag(StatusDrag(status.clone()), |drag, _, _, cx| cx.new(|_| Ghost(status_name(&drag.0).into())))
                .drag_over::<StatusDrag>(move |s, _, _, _| s.border_l_2().border_color(accent))
                .on_drop(cx.listener(move |this, drag: &StatusDrag, _, cx| this.drop_status(drag, &s2, false, cx)))
                .child(status_icon(&status, cx))
                .child(status_name(&status))
                .child(div().text_color(style::tertiary_label(cx)).child(items.len().to_string()))
                .child(div().flex_1())
                .child(
                    Button::new(SharedString::from(format!("add-{group}-{status}")))
                        .icon(icons::symbol("plus"))
                        .ghost()
                        .xsmall()
                        .tooltip(format!("Add a task to {}", status_name(&status)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.adding_in = Some((g3.clone(), s3.clone()));
                            this.column_input.update(cx, |c, cx| {
                                c.set_value("", window, cx);
                                c.focus(window, cx);
                                crate::web_panels::keyboard_to_app(window);
                            });
                            cx.notify();
                        })),
                );
            let mut column = v_flex()
                .id(SharedString::from(format!("column-{group}-{status}")))
                .flex_none()
                .w(px(280.))
                .min_h(px(160.))
                .p(px(6.))
                .rounded(px(10.))
                .bg(style::hover_fill(cx))
                .drag_over::<TaskDrag>(move |s, _, _, _| s.bg(soft))
                .on_drop(cx.listener(move |this, drag: &TaskDrag, _, cx| this.drop_task(drag, &s1, &g1, None, cx)))
                .child(header);
            if adding {
                column = column.child(div().mb(px(6.)).child(Input::new(&self.column_input).small().disabled(self.busy)));
            }
            for task in &items {
                column = column.child(self.card(task, group, cx));
            }
            if items.is_empty() && !adding {
                column = column.child(div().py(px(18.)).text_center().text_size(px(12.)).text_color(style::tertiary_label(cx)).child("Drop tasks here"));
            }
            columns = columns.child(column);
        }
        columns.into_any_element()
    }
    fn board_view(&self, rows: &[TaskRow], cx: &mut Context<Self>) -> AnyElement {
        if !self.by_project() {
            return self.board_columns("all", rows, cx);
        }
        let mut out = v_flex();
        for group in self.groups(rows) {
            let count = rows.iter().filter(|r| project_key(&r.project) == group).count();
            out = out.child(self.project_header(&group, count, cx));
            if !self.collapsed.contains(&format!("project:{group}")) {
                out = out.child(self.board_columns(&group, rows, cx));
            }
        }
        out.into_any_element()
    }
    fn display_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let page = cx.entity().downgrade();
        let (prefs, filter, view, show_done) = (self.prefs.clone(), self.filter.clone(), self.view_name(), self.show_done);
        let mut filters = vec![("All Projects".to_string(), json!("all"))];
        filters.extend(self.project_choices());
        Button::new("tasks-display")
            .label("Display")
            .small()
            .outline()
            .tooltip("Project, grouping, order and the default view")
            .dropdown_menu(move |menu: PopupMenu, window, cx| {
                let mut menu = menu;
                let (p, choices, f) = (page.clone(), filters.clone(), filter.clone());
                menu = menu.submenu_with_icon(Some(icons::symbol("square.stack")), "Project", window, cx, move |sub, _, _| {
                    let mut sub = sub;
                    for (label, value) in &choices {
                        let (p, value) = (p.clone(), value.clone());
                        sub = sub.item(PopupMenuItem::new(label.clone()).checked(f == value).on_click(move |_, _, cx| {
                            p.update(cx, |p, cx| {
                                p.filter = value.clone();
                                p.project = value.is_object().then(|| value.clone());
                                cx.notify();
                            })
                            .ok();
                        }));
                    }
                    sub
                });
                let (p, grouping) = (page.clone(), prefs["grouping"].as_str().unwrap_or("status").to_string());
                menu = menu.submenu_with_icon(Some(icons::symbol("list.bullet")), "Grouping", window, cx, move |sub, _, _| {
                    let mut sub = sub;
                    for (label, value) in [("Status", "status"), ("Project", "project")] {
                        let p = p.clone();
                        sub = sub.item(PopupMenuItem::new(label).checked(grouping == value).on_click(move |_, _, cx| {
                            p.update(cx, |p, cx| p.set_pref("grouping", json!(value), cx)).ok();
                        }));
                    }
                    sub
                });
                let (p, sort) = (page.clone(), prefs["sort"].as_str().unwrap_or("manual").to_string());
                menu = menu.submenu_with_icon(Some(icons::symbol("arrow.up.arrow.down")), "Ordering", window, cx, move |sub, _, _| {
                    let mut sub = sub;
                    for (label, value) in [("Manual", "manual"), ("Newest", "newest"), ("Oldest", "oldest"), ("Title", "title")] {
                        let p = p.clone();
                        sub = sub.item(PopupMenuItem::new(label).checked(sort == value).on_click(move |_, _, cx| {
                            p.update(cx, |p, cx| p.set_pref("sort", json!(value), cx)).ok();
                        }));
                    }
                    sub
                });
                menu = menu.separator();
                if view == "list" {
                    let p = page.clone();
                    menu = menu.item(PopupMenuItem::new("Show Done Tasks").checked(show_done).on_click(move |_, _, cx| {
                        p.update(cx, |p, cx| {
                            p.show_done = !p.show_done;
                            cx.notify();
                        })
                        .ok();
                    }));
                }
                let (p, v) = (page.clone(), view.clone());
                let is_default = prefs["defaultView"].as_str().unwrap_or("list") == view;
                let name = if view == "list" { "List" } else { "Board" };
                menu.item(PopupMenuItem::new(format!("Open Tasks in {name}")).checked(is_default).on_click(move |_, _, cx| {
                    p.update(cx, |p, cx| p.set_pref("defaultView", json!(v), cx)).ok();
                }))
            })
            .into_any_element()
    }
    fn choose_photo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Choose".into()) });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await {
                if let Some(path) = paths.first() {
                    let path = path.to_string_lossy().to_string();
                    this.update(cx, |this, cx| this.change("tasks.photoFile", json!({"path": path}), cx)).ok();
                }
            }
        })
        .detach();
    }
    fn customize_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self.prefs["backgroundColor"].as_str().unwrap_or("").to_string();
        let mut swatches = h_flex().gap(px(6.)).flex_wrap();
        for (i, color) in ["", "#f6f4ef", "#eef3f8", "#f3eef7", "#1b1d22", "#14211d", "#221a17"].into_iter().enumerate() {
            let fill = u32::from_str_radix(color.trim_start_matches('#'), 16).ok().map(|c| Hsla::from(rgb(c)));
            let chosen = current == color;
            swatches = swatches.child(
                div()
                    .id(("tasks-swatch", i))
                    .role(Role::Button)
                    .aria_label(SharedString::from(if color.is_empty() { "Use the theme background".to_string() } else { format!("Use {color}") }))
                    .aria_selected(chosen)
                    .size(px(24.))
                    .rounded_full()
                    .border_1()
                    .border_color(if chosen { style::accent(cx) } else { style::card_border(cx) })
                    .when(chosen, |d| d.border_2())
                    .bg(fill.unwrap_or(cx.theme().background))
                    .when(fill.is_none(), |d| d.child(div().size_full().flex().items_center().justify_center().child(icons::symbol("circle.lefthalf.filled").with_size(px(14.)).text_color(style::secondary_label(cx)))))
                    .cursor_pointer()
                    .tip(if color.is_empty() { "Use the theme background".to_string() } else { format!("Use {color}") })
                    .on_click(cx.listener(move |this, _, _, cx| this.set_pref("backgroundColor", json!(color), cx))),
            );
        }
        let has_photo = self.photo.is_some();
        let mut panel = v_flex()
            .id("tasks-customize")
            .w(px(300.))
            .gap(px(10.))
            .p(px(12.))
            .rounded(px(10.))
            .border_1()
            .border_color(style::card_border(cx))
            .bg(cx.theme().background)
            .shadow_lg()
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                if let Some(path) = paths.paths().first() {
                    this.change("tasks.photoFile", json!({"path":path.to_string_lossy()}), cx);
                }
            }))
            .child(div().text_size(px(12.)).text_color(style::secondary_label(cx)).child("Color"))
            .child(swatches)
            .child(h_flex().gap(px(6.)).child(div().flex_1().child(Input::new(&self.color).small().disabled(self.busy))).child(Button::new("tasks-color").label("Apply").small().outline().tooltip("Use this hex color, like #1b1d22").disabled(self.busy).on_click(cx.listener(|this, _, _, cx| {
                let value = this.color.read(cx).value().to_string();
                this.set_pref("backgroundColor", json!(value), cx);
            }))))
            .child(div().text_size(px(12.)).text_color(style::secondary_label(cx)).child("Photo"))
            .child(
                h_flex()
                    .gap(px(6.))
                    .child(div().flex_1().child(Button::new("tasks-photo").label(if has_photo { "Change Photo…" } else { "Choose Photo…" }).small().outline().w_full().tooltip("A PNG, JPEG, WebP or GIF under 10 MB. You can drop one here too.").disabled(self.busy).on_click(cx.listener(|this, _, window, cx| this.choose_photo(window, cx)))))
                    .when(has_photo, |d| d.child(Button::new("tasks-reset-background").label("Remove").small().outline().tooltip("Remove the photo").disabled(self.busy).on_click(cx.listener(|this, _, _, cx| this.set_pref("backgroundPhoto", Value::Null, cx))))),
            );
        if has_photo {
            let mut fades = h_flex().gap(px(4.));
            for (label, value) in [("Clear", 0.0), ("Light", 0.4), ("Soft", 0.7), ("Faint", 0.9)] {
                let on = (self.prefs["fade"].as_f64().unwrap_or(0.85) - value).abs() < 0.01;
                fades = fades.child(Button::new(SharedString::from(format!("fade-{label}"))).label(label).xsmall().selected(on).tooltip("How much the photo fades").on_click(cx.listener(move |this, _, _, cx| this.set_pref("fade", json!(value), cx))));
            }
            let mut places = h_flex().gap(px(4.));
            for place in ["top", "center", "bottom"] {
                let on = self.prefs["photoPosition"].as_str().unwrap_or("center") == place;
                let label = format!("{}{}", place[..1].to_uppercase(), &place[1..]);
                places = places.child(Button::new(SharedString::from(format!("place-{place}"))).label(label).xsmall().selected(on).tooltip(format!("Show the {place} of the photo")).on_click(cx.listener(move |this, _, _, cx| this.set_pref("photoPosition", json!(place), cx))));
            }
            panel = panel.child(div().text_size(px(12.)).text_color(style::secondary_label(cx)).child("Fade")).child(fades).child(places);
        }
        panel.into_any_element()
    }
    fn editor_sheet(&self, cx: &mut Context<Self>) -> AnyElement {
        let page = cx.entity().downgrade();
        let status = self.status.clone();
        let statuses = self.board_order();
        let status_button = Button::new("task-status").label(status_name(&status)).icon(status_icon(&status, cx)).small().outline().tooltip("Status").dropdown_menu(move |menu: PopupMenu, _, cx| {
            let mut menu = menu;
            for s in &statuses {
                let (p, s) = (page.clone(), s.clone());
                menu = menu.item(PopupMenuItem::new(status_name(&s)).icon(status_icon(&s, cx)).checked(status == s).on_click(move |_, _, cx| {
                    p.update(cx, |p, cx| {
                        p.status = s.clone();
                        cx.notify();
                    })
                    .ok();
                }));
            }
            menu
        });
        let page = cx.entity().downgrade();
        let selected = self.project.clone().unwrap_or(Value::Null);
        let choices = self.project_choices();
        let project_button = Button::new("task-project").label(self.project_name(&selected)).icon(icons::symbol("square.stack")).small().outline().tooltip("Project").dropdown_menu(move |menu: PopupMenu, _, _| {
            let mut menu = menu;
            for (label, value) in &choices {
                let (p, value) = (page.clone(), value.clone());
                menu = menu.item(PopupMenuItem::new(label.clone()).checked(value == selected).on_click(move |_, _, cx| {
                    p.update(cx, |p, cx| {
                        p.project = (!value.is_null()).then(|| value.clone());
                        cx.notify();
                    })
                    .ok();
                }));
            }
            menu
        });
        let app = self.app.clone();
        let editing = self.editing_baseline.as_ref().map(|r| r.id.clone());
        let sheet = v_flex()
            .id("task-editor")
            .role(Role::Dialog)
            .aria_label(if editing.is_some() { "Edit task" } else { "New task" })
            .w(px(600.))
            .max_w_full()
            .gap(px(8.))
            .p(px(16.))
            .rounded(px(12.))
            .bg(cx.theme().background)
            .border_1()
            .border_color(style::card_border(cx))
            .shadow_lg()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(h_flex().child(div().flex_1().text_size(px(12.)).text_color(style::secondary_label(cx)).child(if editing.is_some() { "Edit task" } else { "New task" })).child(Button::new("task-editor-close").icon(icons::symbol("xmark")).ghost().xsmall().tooltip("Close (Esc)").on_click(cx.listener(|this, _, window, cx| this.close_editor(window, cx)))))
            .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child(Input::new(&self.title).appearance(false).disabled(self.busy)))
            .child(Textarea::new(&self.notes).appearance(false).disabled(self.busy))
            .child(
                h_flex()
                    .gap(px(6.))
                    .child(status_button)
                    .child(project_button)
                    .child(Button::new("task-new-project").label("New Project…").small().ghost().tooltip("Create a Threadbear project, then pick it here").on_click(move |_, window, cx| {
                        app.update(cx, |app, cx| app.new_project(None, window, cx)).ok();
                    })),
            )
            .child(div().h(px(1.)).bg(style::hairline(cx)))
            .child(
                h_flex()
                    .gap(px(8.))
                    .when_some(editing, |d, id| d.child(Button::new("task-delete").label("Delete").small().ghost().tooltip("Delete the task. You can undo it.").on_click(cx.listener(move |this, _, window, cx| {
                        this.delete(&id, cx);
                        this.close_editor(window, cx);
                    }))))
                    .child(div().flex_1())
                    .when_some(self.error.clone(), |d, e| d.child(div().text_size(px(12.)).text_color(style::failed_text(cx)).child(e.clone().test_text(e))))
                    .child(Button::new("task-cancel").label("Cancel").small().outline().tooltip("Close without saving").on_click(cx.listener(|this, _, window, cx| this.close_editor(window, cx))))
                    .child(Button::new("save-task").label("Save").small().primary().loading(self.busy).tooltip("Save the task to your account").disabled(self.busy).on_click(cx.listener(|this, _, window, cx| this.save(window, cx)))),
            );
        div()
            .id("task-editor-backdrop")
            .a11y_role(Role::GenericContainer)
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(90.))
            .bg(gpui_kit::black().opacity(0.3))
            .on_click(cx.listener(|this, _, window, cx| {
                if !this.draft_dirty_editor(cx) {
                    this.close_editor(window, cx);
                }
            }))
            .child(sheet)
            .into_any_element()
    }
}

impl Render for TasksPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view_name();
        let rows = self.visible();
        let open = rows.iter().filter(|r| r.status != "done").count();
        let title = if self.filter.is_object() { self.project_name(&self.filter) } else { "Tasks".into() };
        let mut segment = h_flex().gap(px(2.)).p(px(2.)).rounded(px(7.)).bg(style::hover_fill(cx));
        for name in ["list", "board"] {
            segment = segment.child(Button::new(name).label(if name == "list" { "List" } else { "Board" }).small().ghost().selected(view == name).tooltip(if name == "list" { "Show the list" } else { "Show the board" }).on_click(cx.listener(move |this, _, window, cx| {
                this.view = Some(name.into());
                this.focus_composer(window, cx);
                cx.notify();
            })));
        }
        let mut actions = h_flex().flex_1().min_w_0().flex_wrap().justify_end().gap(px(6.)).child(segment.flex_none()).child(self.display_menu(cx));
        actions = actions.child(Button::new("tasks-customize").label("Background").small().outline().selected(self.customize).tooltip("Background color and photo").on_click(cx.listener(|this, _, window, cx| {
            this.customize = !this.customize;
            if this.customize {
                let color = this.prefs["backgroundColor"].as_str().unwrap_or("").to_string();
                this.color.update(cx, |f, cx| f.set_value(color, window, cx));
            }
            cx.notify();
        })));
        if let Some(url) = self.app.read_with(cx, |app, _| app.website_url.clone()).ok().flatten() {
            actions = actions.child(Button::new("tasks-web").icon(icons::symbol("safari")).small().ghost().tooltip("Open Tasks on the web, synced with this Mac").on_click(move |_, _, cx| {
                crate::open_gate::open_web(&format!("{}/tasks", url.trim_end_matches('/')), cx);
            }));
        }
        let app = self.app.clone();
        actions = actions.child(Button::new("tasks-close").label("Close").small().ghost().tooltip("Close Tasks (Esc)").on_click(move |_, _, cx| {
            app.update(cx, |app, cx| {
                app.close_tasks(cx);
            })
            .ok();
        }));
        let header = h_flex()
            .gap(px(10.))
            .mb(px(18.))
            .when(view == "list", |d| d.w_full().max_w(px(880.)).mx_auto())
            .child(div().text_size(px(20.)).font_weight(FontWeight::SEMIBOLD).child(title.clone().test_text(title)))
            .child(div().text_size(px(13.)).text_color(style::tertiary_label(cx)).child(format!("{open} open")))
            .child(actions);

        let body = if view == "list" { self.list_view(&rows, cx) } else { self.board_view(&rows, cx) };
        let mut page = v_flex()
            .id("tasks-content")
            .key_context("TasksPage")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &CloseTasks, window, cx| {
                if this.editor_open {
                    this.close_editor(window, cx);
                    cx.notify();
                    return;
                }
                let app = this.app.clone();
                cx.defer(move |cx| {
                    app.update(cx, |app, cx| {
                        app.close_tasks(cx);
                    })
                    .ok();
                });
            }))
            .size_full()
            .px(px(28.))
            .pt(px(24.))
            .text_color(style::label_text(cx))
            .child(header);
        if let Some(error) = self.error.clone().filter(|_| !self.editor_open) {
            page = page.child(div().mb(px(10.)).text_size(px(13.)).text_color(style::failed_text(cx)).child(error.clone().test_text(error)));
        }
        page = page.child(div().id("tasks-scroll").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.scroll).pb(px(80.)).child(body));

        let bg = self.prefs["backgroundColor"].as_str().and_then(|c| u32::from_str_radix(c.trim_start_matches('#'), 16).ok()).map(|c| Hsla::from(rgb(c))).unwrap_or(cx.theme().background);
        // The page, the toast and the editor sheet are all read by the scripted control.
        let mut shell = div().id("tasks-page").test_support().relative().size_full().overflow_hidden().bg(bg);
        if let Some(url) = &self.photo {
            let resource = Resource::Uri(url.clone().into());
            if let Some(Ok(image)) = window.use_asset::<ImgResourceLoader>(&resource, cx) {
                let size = image.size(0);
                let y = match self.prefs["photoPosition"].as_str() {
                    Some("top") => 0.,
                    Some("bottom") => 1.,
                    _ => 0.5,
                };
                let (x, y, w, h) = threadbear_core::looks::BackdropFocus::new(0.5, y).frame((u32::from(size.width) as f32, u32::from(size.height) as f32), self.background_size);
                shell = shell.child(img(url.clone()).absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).object_fit(ObjectFit::Fill).opacity((1. - self.prefs["fade"].as_f64().unwrap_or(0.85)) as f32));
            }
            let page = cx.entity().downgrade();
            shell = shell.child(
                canvas(
                    move |bounds, _, cx| {
                        let one = style::zoomed(1.);
                        let size = (f32::from(bounds.size.width) / one, f32::from(bounds.size.height) / one);
                        let page = page.clone();
                        cx.defer(move |cx| {
                            page.update(cx, |p, cx| {
                                if (p.background_size.0 - size.0).abs() > 0.5 || (p.background_size.1 - size.1).abs() > 0.5 {
                                    p.background_size = size;
                                    cx.notify();
                                }
                            })
                            .ok();
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .inset_0()
                .size_full(),
            );
        }
        shell = shell.child(page.bg(gpui_kit::transparent_black()));
        if self.customize {
            shell = shell.child(div().absolute().top(px(64.)).right(px(28.)).child(self.customize_panel(cx)));
        }
        if let Some((text, _, _)) = self.toast.clone() {
            shell = shell.child(
                div().absolute().bottom(px(24.)).left_0().right_0().flex().justify_center().child(
                    h_flex()
                        .gap(px(14.))
                        .pl(px(14.))
                        .pr(px(6.))
                        .py(px(6.))
                        .rounded(px(10.))
                        .bg(cx.theme().background)
                        .border_1()
                        .border_color(style::card_border(cx))
                        .text_color(style::label_text(cx))
                        .text_size(px(13.))
                        .shadow_lg()
                        .child(text.clone().test_text(text))
                        .child(Button::new("undo-task").label("Undo").small().outline().tooltip("Undo the last change").on_click(cx.listener(|this, _, _, cx| this.undo(cx)))),
                ),
            );
        }
        if self.editor_open {
            shell = shell.child(self.editor_sheet(cx));
        }
        shell
    }
}

/// Scripted commands enter through the same paths as clicks and typing.
pub(crate) fn automation(app: &Entity<Threadbear>, cmd: &Value, window: &mut Window, cx: &mut App) -> Result<Value, String> {
    if app.read(cx).tasks_page.is_none() {
        app.update(cx, |app, cx| app.toggle_tasks(window, cx));
    }
    let page = app.read(cx).tasks_page.clone().ok_or("Tasks experiment is off")?;
    page.update(cx, |p, cx| {
        // `quick` types into the list's quick-add box and presses Return.
        if let Some(text) = cmd["quick"].as_str() {
            p.composer.update(cx, |f, cx| f.set_value(text.to_owned(), window, cx));
            p.submit_composer(window, cx);
        }
        if cmd["retryQuick"].as_bool() == Some(true) {
            p.submit_composer(window, cx);
        }
        if let Some(title) = cmd["title"].as_str() {
            p.editor_open = true;
            p.title.update(cx, |f, cx| f.set_value(title.to_owned(), window, cx));
        }
        if let Some(notes) = cmd["notes"].as_str() {
            p.notes.update(cx, |f, cx| f.set_value(notes.to_owned(), window, cx));
        }
        if cmd["save"].as_bool() == Some(true) {
            p.save(window, cx);
        }
        if let Some(title) = cmd["open"].as_str() {
            if let Some(row) = p.rows.iter().find(|r| r.title == title || r.id == title).cloned() {
                p.open_editor(&row, false, window, cx);
            }
        }
        if cmd["makeDefault"].as_bool() == Some(true) {
            let view = p.view_name();
            p.set_pref("defaultView", json!(view), cx);
        }
        // Drags, as a drop would do them: a task, a status section or column, a project group.
        if let (Some(id), Some(status)) = (cmd["moveTask"].as_str(), cmd["status"].as_str()) {
            let title = p.rows.iter().find(|r| r.id == id || r.title == id).map(|r| (r.id.clone(), r.title.clone()));
            if let Some((id, title)) = title {
                let group = cmd["group"].as_str().unwrap_or("all").to_string();
                p.drop_task(&TaskDrag { id, title }, status, &group, cmd["before"].as_str(), cx);
            }
        }
        if let (Some(from), Some(to)) = (cmd["moveStatus"].as_str(), cmd["to"].as_str()) {
            let list = p.view_name() == "list";
            p.drop_status(&StatusDrag(from.into()), to, list, cx);
        }
        if let (Some(from), Some(to)) = (cmd["moveProject"].as_str(), cmd["to"].as_str()) {
            p.drop_project(&ProjectDrag(from.into(), String::new()), to, cx);
        }
        if let Some(view) = cmd["view"].as_str() {
            p.view = Some(view.into());
        }
        if let Some(on) = cmd["showDone"].as_bool() {
            p.show_done = on;
        }
        cx.notify();
    });
    Ok(json!("ok"))
}
