// Cloud-owned Tasks: a quick-add list first, a board second. Row edits show at
// once and roll back with the error; the editor keeps its draft through a
// failed save, so nothing typed is lost to a dropped connection.
use crate::tips::Tip as _;
use crate::a11y::A11yRole as _;
use crate::{app::Threadbear, call, icons, style::{self, px}};
use gpui_kit::base::{TestSupportExt as _, TestTextExt as _};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::{HashSet, HashMap};
use std::{cell::RefCell, rc::Rc};
use threadbear_core::task_order;

actions!(tasks_page, [CloseTasks]);

#[path = "task_detail.rs"]
mod detail;
use detail::{due_chip, priority_icon, subtask_chip};

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskRow {
    id: String,
    title: String,
    notes: String,
    status: String,
    #[serde(default)]
    column_id: Option<String>,
    #[serde(default)]
    thread: Option<Value>,
    #[serde(default)]
    thread_command_id: Option<String>,
    #[serde(default)]
    thread_error: Option<String>,
    project: Option<Value>,
    position: f64,
    created_at: f64,
    // Newer runners' fields; an older one leaves them out.
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    due_date: Option<String>,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default, deserialize_with = "links_or_none")]
    links: Vec<Value>,
    #[serde(default)]
    comment_count: Option<u64>,
    #[serde(default)]
    created_by: Option<Value>,
}
fn links_or_none<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Value>, D::Error> {
    let value: Value = serde::Deserialize::deserialize(d)?;
    Ok(value.as_array().cloned().unwrap_or_default())
}
fn column_id(row: &TaskRow) -> &str { row.column_id.as_deref().unwrap_or(&row.status) }
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
        let wrap = self.new_thread.prefs.tasks_wrap_columns;
        page.update(cx, |p, cx| {
            p.wrap_columns = wrap;
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
            "selectedTask":page.and_then(|p| p.selected_task.clone()),
            "hasPhoto":page.is_some_and(|p| p.photo.is_some()),
            "photoLoaded":page.is_some_and(|p| p.photo_image.is_some()),
            "quickDraft":page.map(|p| p.composer.read(cx).value().to_string()),
            "toast":page.and_then(|p| p.toast.as_ref().map(|t| t.0.clone())),
            "tasks":page.map(|p| p.rows.iter().map(|t| json!({"id":t.id,"title":t.title,"status":t.status,"columnId":column_id(t),"thread":t.thread,"notes":t.notes,"project":project_key(&t.project),
                "priority":t.priority,"dueDate":t.due_date,"parentId":t.parent_id,"links":t.links,"commentCount":t.comment_count})).collect::<Vec<_>>()),
            "topLevel":page.map(|p| p.visible().iter().map(|t| t.title.clone()).collect::<Vec<_>>()),
            "wrapColumns":page.map(|p| p.wrap_columns),
            "detail":page.filter(|p| p.editor_open && p.editing_baseline.is_some()).map(|p| json!({"task":p.editing,"picker":p.detail.picker,"commentsOnly":p.detail.comments_only,
                "activity":match &p.detail.activity { Some(Ok(items)) => json!(items.iter().map(|i| json!({"kind":i["kind"],"body":i["body"]})).collect::<Vec<_>>()), Some(Err(e)) => json!({"error":e}), None => Value::Null },
                "related":p.detail.related.as_ref().map(|r| r.clone().unwrap_or_else(|e| json!({"error":e}))),"error":p.detail.error,"month":[p.detail.month.0,p.detail.month.1]})),
            "error":page.and_then(|p| p.error.clone())})
    }
    pub(crate) fn render_tasks_page(&self) -> AnyElement {
        match &self.tasks_page {
            Some(page) => div().size_full().child(page.clone()).into_any_element(),
            None => div().into_any_element(),
        }
    }
}

/// A highlight's way back from a task: its page or link, else the thread that has it.
pub(crate) fn link_to_highlight(item: &threadbear_core::highlights::Highlight, thread: &threadbear_core::inbox::ThreadSummary) -> Value {
    match &item.url {
        Some(url) if !url.starts_with("file://") => json!({"kind":"link","title":item.title,"url":url}),
        _ => json!({"kind":"thread","title":thread.title,"thread":{"runnerId":thread.runner_id,"id":thread.id}}),
    }
}

impl Threadbear {
    /// Highlights' Add to Tasks: a task named after the row, linking back to
    /// it, with a short note when it's in (or the error, as Tasks is off).
    pub(crate) fn add_task_from(&mut self, title: String, mut link: Value, cx: &mut Context<Self>) {
        if !self.experiment_on("tasks") {
            self.show_error("Tasks is off. Turn it on in Settings, General, Experiments.".into(), cx);
            return;
        }
        if link["kind"] == "thread" && link["thread"]["runnerId"].is_null() {
            // This Mac's own thread: its runner, as Tasks names runners.
            let own = self.remote.all_runners.iter().find(|r| r.is_self).map(|r| r.id.clone()).unwrap_or_else(|| "local".into());
            link["thread"]["runnerId"] = json!(own);
        }
        link["id"] = json!(threadbear_core::notes::new_id());
        let args = json!({"id": threadbear_core::notes::new_id(), "title": title, "notes": "", "status": "todo", "columnId": "todo", "project": Value::Null, "position": -(crate::now_ms() as f64), "links": [link]});
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "tasks.create", args).await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(_) => {
                        this.show_import_note(format!("Added to Tasks: {title}"), cx);
                        this.tasks_updated(cx);
                    }
                    Err(e) => this.show_error(format!("Couldn't add {title} to Tasks: {}", e.message), cx),
                }
            }).ok();
        }).detach();
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
    composer: Entity<TextareaState>,
    column_input: Entity<TextareaState>,
    adding_in: Option<(String, String)>,
    projects: Vec<Value>,
    filter: Value,
    photo: Option<String>,
    photo_image: Option<(std::sync::Arc<Image>, (f32, f32))>,
    customize: bool,
    refresh_again: bool,
    draft_id: String,
    project_filter_id: Option<String>,
    focus: FocusHandle,
    selected_task: Option<String>,
    editing: Option<String>,
    editing_baseline: Option<TaskRow>,
    status: String,
    project: Option<Value>,
    editor_open: bool,
    send_after_save: bool,
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
    scrolls: RefCell<HashMap<String, ScrollHandle>>,
    task_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    viewport_height: f32,
    background_size: (f32, f32),
    // The photo's fade and place: drawn live while dragged, saved on release.
    fade_slider: Entity<SliderState>,
    place_slider: Entity<SliderState>,
    // The open task's page (task_detail.rs).
    detail: detail::Detail,
    detail_scroll: ScrollHandle,
    comment_input: Entity<TextareaState>,
    sub_input: Entity<InputState>,
    link_url: Entity<InputState>,
    link_title: Entity<InputState>,
    // A save from the task's page keeps it open.
    keep_open: bool,
    wrap_columns: bool,
    _subscriptions: Vec<Subscription>,
}

/// Top, center and bottom as stops on one slider, as Convex keeps them.
fn place_value(place: &str) -> f32 {
    match place { "top" => 0., "bottom" => 1., _ => 0.5 }
}
fn place_name(value: f32) -> &'static str {
    if value < 0.25 { "top" } else if value > 0.75 { "bottom" } else { "center" }
}

impl TasksPage {
    fn new(app: WeakEntity<Threadbear>, client: std::sync::Arc<threadbear_client::runner::Client>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        static BOUND: std::sync::Once = std::sync::Once::new();
        BOUND.call_once(|| cx.bind_keys([KeyBinding::new("escape", CloseTasks, Some("TasksPage"))]));
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        crate::web_panels::keyboard_to_app(window);
        let composer = cx.new(|cx| TextareaState::new(window, cx).placeholder("Add a task. Return to add, Shift-Return for a new line").auto_grow(1, 6).submit_on_enter(true));
        // Two lines tall to start, so a card's title has room as it's typed.
        let column_input = cx.new(|cx| TextareaState::new(window, cx).placeholder("Task title, then Return").auto_grow(2, 6).submit_on_enter(true));
        let fade_slider = cx.new(|_| SliderState::new().min(0.).max(0.95).step(0.01).default_value(0.85_f32));
        let title = cx.new(|cx| InputState::new(window, cx).placeholder("Task title"));
        let notes = cx.new(|cx| TextareaState::new(window, cx).placeholder("Add a description…").auto_grow(3, 20));
        let comment_input = cx.new(|cx| TextareaState::new(window, cx).placeholder("Leave a comment. Return to send, Shift-Return for a new line").auto_grow(1, 8).submit_on_enter(true));
        let sub_input = cx.new(|cx| InputState::new(window, cx).placeholder("Sub-task title, then Return"));
        let link_url = cx.new(|cx| InputState::new(window, cx).placeholder("https://"));
        let link_title = cx.new(|cx| InputState::new(window, cx).placeholder("Title (optional)"));
        let place_slider = cx.new(|_| SliderState::new().min(0.).max(1.).step(0.5).default_value(0.5_f32));
        let subs = vec![
            cx.subscribe(&fade_slider, |this, _, ev: &SliderEvent, cx| match ev {
                SliderEvent::Change(v) => {
                    this.prefs["fade"] = json!((v.start() * 100.).round() / 100.);
                    cx.notify();
                }
                SliderEvent::Release(v) => this.set_pref("fade", json!((v.start() * 100.).round() / 100.), cx),
            }),
            cx.subscribe(&place_slider, |this, _, ev: &SliderEvent, cx| match ev {
                SliderEvent::Change(v) => {
                    this.prefs["photoPosition"] = json!(place_name(v.start()));
                    cx.notify();
                }
                SliderEvent::Release(v) => this.set_pref("photoPosition", json!(place_name(v.start())), cx),
            }),
            cx.subscribe_in(&composer, window, |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { shift: false, .. } = ev {
                    this.submit_composer(window, cx);
                }
            }),
            // The task's page saves its title and description as you leave them.
            cx.subscribe_in(&title, window, |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) { this.autosave(window, cx); }
            }),
            cx.subscribe_in(&notes, window, |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::Blur) { this.autosave(window, cx); }
            }),
            cx.subscribe_in(&comment_input, window, |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { shift: false, .. } = ev { this.send_comment(window, cx); }
            }),
            cx.subscribe_in(&sub_input, window, |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { .. } => this.add_subtask(window, cx),
                InputEvent::Blur if this.sub_input.read(cx).value().trim().is_empty() => { this.detail.adding_sub = false; cx.notify(); }
                _ => {}
            }),
            cx.subscribe_in(&link_url, window, |this, _, ev: &InputEvent, window, cx| if let InputEvent::PressEnter { .. } = ev { this.add_link(window, cx); }),
            cx.subscribe_in(&link_title, window, |this, _, ev: &InputEvent, window, cx| if let InputEvent::PressEnter { .. } = ev { this.add_link(window, cx); }),
            cx.subscribe_in(&column_input, window, |this, _, ev: &InputEvent, window, cx| match ev {
                InputEvent::PressEnter { shift: false, .. } => this.submit_column(window, cx),
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
            title,
            notes,
            composer,
            column_input,
            adding_in: None,
            projects: vec![],
            filter: json!("all"),
            photo: None,
            photo_image: None,
            customize: false,
            refresh_again: false,
            draft_id: threadbear_core::notes::new_id(),
            project_filter_id: None,
            focus,
            selected_task: None,
            editing: None,
            editing_baseline: None,
            status: "todo".into(),
            project: None,
            editor_open: false,
            send_after_save: false,
            view: None,
            busy: false,
            loading: false,
            error: None,
            show_done: false,
            collapsed: HashSet::new(),
            toast: None,
            toast_timer: None,
            scroll: ScrollHandle::new(),
            scrolls: RefCell::new(HashMap::new()),
            task_bounds: Rc::new(RefCell::new(HashMap::new())),
            viewport_height: 600.,
            background_size: (0., 0.),
            fade_slider,
            place_slider,
            detail: Default::default(),
            detail_scroll: ScrollHandle::new(),
            comment_input,
            sub_input,
            link_url,
            link_title,
            keep_open: false,
            wrap_columns: false,
            _subscriptions: subs,
        }
    }
    fn view_name(&self) -> String {
        self.view.as_deref().or(self.prefs["defaultView"].as_str()).unwrap_or("list").to_string()
    }
    fn definitions(&self) -> Vec<Value> {
        let mut out = self.prefs["columns"].as_array().cloned().unwrap_or_default();
        for id in task_order::BOARD {
            if !out.iter().any(|c| c["id"] == id) { out.push(json!({"id":id,"name":status_name(id),"status":id})); }
        }
        out
    }
    fn column_name(&self, id: &str) -> String {
        self.definitions().iter().find(|c| c["id"] == id).and_then(|c| c["name"].as_str()).unwrap_or("Todo").to_string()
    }
    fn canonical(&self, id: &str) -> String {
        self.definitions().iter().find(|c| c["id"] == id).and_then(|c| c["status"].as_str()).unwrap_or("todo").to_string()
    }
    fn column_order(&self, list: bool) -> Vec<String> {
        let field = if list { "sectionOrder" } else { "boardColumnOrder" };
        let mut saved = strings(&self.prefs[field]);
        if self.prefs[field].is_null() { saved = task_order::full_order(&strings(&self.prefs[if list { "listOrder" } else { "columnOrder" }]), if list { &task_order::LIST } else { &task_order::BOARD }); }
        let ids: Vec<String> = self.definitions().iter().filter_map(|c| c["id"].as_str().map(String::from)).collect();
        let mut out = Vec::new();
        for id in saved.into_iter().chain(ids.iter().cloned()) { if ids.contains(&id) && !out.contains(&id) { out.push(id); } }
        out
    }
    fn board_order(&self) -> Vec<String> { self.column_order(false) }
    fn list_order(&self) -> Vec<String> { self.column_order(true) }
    fn edit_column(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy { return; }
        let name = id.as_ref().map(|id| self.column_name(id)).unwrap_or_default();
        let field = cx.new(|cx| InputState::new(window, cx).placeholder("Column name").default_value(name));
        let (page, typed) = (cx.entity().downgrade(), field.clone());
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let (page, field, id) = (page.clone(), typed.clone(), id.clone());
            dialog.title(if id.is_some() { "Rename column" } else { "Add column" }).width(px(340.)).child(Input::new(&typed))
                .button_props(DialogButtonProps::default().ok_text("Save").show_cancel(true)).on_ok(move |_, _, cx| {
                    let name = field.read(cx).value().trim().to_owned();
                    if name.is_empty() || name.chars().count() > 80 { return false; }
                    page.update(cx, |this, cx| {
                        let mut columns = this.definitions();
                        if let Some(id) = &id { for c in &mut columns { if c["id"] == *id { c["name"] = json!(name); } } }
                        else if columns.len() < 32 { columns.push(json!({"id":threadbear_core::notes::new_id(),"name":name,"status":"todo"})); }
                        this.set_pref("columns", json!(columns), cx);
                    }).ok();
                    true
                })
        });
        field.update(cx, |f, cx| f.focus(window, cx));
        crate::web_panels::keyboard_to_app(window);
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
                Some(row) => title != row.title || notes != row.notes || self.status != column_id(row) || self.project != row.project,
                None => !title.is_empty() || !notes.is_empty(),
            }
    }
    fn focus_composer(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_name() == "list" && !self.editor_open {
            self.composer.update(cx, |c, cx| c.focus(window, cx));
            crate::web_panels::keyboard_to_app(window);
        }
    }
    // GPUI has no HTTP client in this app. Fetch storage bytes off the UI
    // thread, then use the same in-memory image path as other native pictures.
    fn load_photo(&mut self, url: String, cx: &mut Context<Self>) {
        let source = url.clone();
        let fetch = cx.background_executor().spawn(async move {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_global(Some(std::time::Duration::from_secs(20))).build().into();
            let mut response = agent.get(&source).call().map_err(|e| e.to_string())?;
            let bytes = response.body_mut().with_config().limit(10 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())?;
            let format = crate::viewer::format_of("", &bytes).ok_or("Unsupported picture format")?;
            let reader = image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format().map_err(|e| e.to_string())?;
            let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
            Ok::<_, String>((std::sync::Arc::new(Image::from_bytes(format, bytes)), (w as f32, h as f32)))
        });
        cx.spawn(async move |this, cx| {
            let result = fetch.await;
            this.update(cx, |this, cx| {
                // An earlier request must not replace a newer photo or removal.
                if this.photo.as_deref() != Some(url.as_str()) { return; }
                match result {
                    Ok(image) => this.photo_image = Some(image),
                    Err(_) => this.error = Some("Could not load the background photo. Choose it again to retry.".into()),
                }
                cx.notify();
            }).ok();
        }).detach();
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
                        let photo = photo.as_str().map(String::from);
                        if this.photo != photo {
                            this.photo = photo.clone();
                            this.photo_image = None;
                            if let Some(url) = photo { this.load_photo(url, cx); }
                        }
                        if let Some(id) = this.project_filter_id.take() {
                            if let Some(p) = this.projects.iter().find(|p| p["id"] == id) {
                                this.filter = json!({"runnerId":p["runnerId"],"id":p["id"]});
                                this.project = Some(this.filter.clone());
                            }
                        }
                        this.rows = rows.into_iter().filter_map(|r| serde_json::from_value(r).ok()).collect();
                        if this.editor_open && this.editing_baseline.is_some() {
                            this.load_activity(cx);
                            this.load_images(cx);
                        }
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
                row.column_id = Some(patch["columnId"].as_str().unwrap_or(s).into());
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
            if patch.get("priority").is_some() {
                row.priority = patch["priority"].as_str().map(String::from);
            }
            if patch.get("dueDate").is_some() {
                row.due_date = patch["dueDate"].as_str().map(String::from);
            }
            if let Some(links) = patch["links"].as_array() {
                row.links = links.clone();
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
        let undo = (next == "done").then(|| (format!("Done: {}", task.title), json!({"id": task.id, "status": task.status,"columnId":column_id(task)})));
        self.patch(&task.id.clone(), json!({"status": next}), undo, cx);
    }
    fn ask_delete(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy { return; }
        let Some(title) = self.rows.iter().find(|r| r.id == id).map(|r| r.title.clone()) else { return };
        let page = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (page, id) = (page.clone(), id.clone());
            alert.title(format!("Delete {title}?"))
                .description("You can undo this afterward.")
                .button_props(DialogButtonProps::default().ok_text("Delete task").cancel_text("Keep task").show_cancel(true))
                .on_ok(move |_, window, cx| {
                    page.update(cx, |this, cx| {
                        this.delete(&id, cx);
                        if this.editing.as_deref() == Some(id.as_str()) { this.close_editor(window, cx); }
                    }).ok();
                    true
                })
        });
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
        let position = self.rows.iter().filter(|r| column_id(r) == status).map(|r| r.position).min_by(f64::total_cmp).map_or(crate::now_ms() as f64, |p| p - 1000.);
        let id = threadbear_core::notes::new_id();
        self.rows.push(TaskRow { id: id.clone(), title: title.clone(), notes: notes.clone(), status: self.canonical(status), column_id: Some(status.into()), thread: None, thread_command_id: None, thread_error: None, project: (!project.is_null()).then(|| project.clone()), position, created_at: crate::now_ms() as f64, priority: None, due_date: None, parent_id: None, links: vec![], comment_count: None, created_by: None });
        self.busy = true;
        self.error = None;
        cx.notify();
        let args = json!({"id": id, "title": title, "notes": notes, "status": self.canonical(status), "columnId":status, "project": project, "position": position});
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
        let mut args = json!({"title": title, "notes": self.notes.read(cx).value().to_string(), "status": self.canonical(&self.status), "columnId":self.status, "project": self.project});
        let method = if self.editing_baseline.is_some() { "tasks.update" } else { "tasks.create" };
        if let Some(base) = &self.editing_baseline {
            // Send only what changed here, so another device's edits survive.
            let original = json!({"title":base.title,"notes":base.notes,"status":base.status,"columnId":column_id(base),"project":base.project});
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
        let saved = args["id"].as_str().unwrap_or("").to_string();
        let (sent_title, sent_notes) = (args["title"].as_str().map(String::from), args["notes"].as_str().map(String::from));
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, method, args).await;
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                match result {
                    Ok(_) => {
                        if this.send_after_save {
                            this.send_after_save = false;
                            let id = saved.clone();
                            this.change("tasks.startThread", json!({"id":id}), cx);
                        }
                        // A new task's sheet closes; the task's own page stays,
                        // with what was saved as its new starting point.
                        if method == "tasks.create" && !this.keep_open {
                            this.close_editor(window, cx);
                        } else if this.editing.as_deref() == Some(saved.as_str()) {
                            if let Some(base) = this.editing_baseline.as_mut() {
                                if let Some(t) = sent_title { base.title = t; }
                                if let Some(n) = sent_notes { base.notes = n; }
                            }
                        }
                        this.keep_open = false;
                        this.refresh(cx);
                    }
                    Err(e) => { this.send_after_save = false; this.keep_open = false; this.error = Some(e.message); },
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
        self.detail.picker = None;
        self.detail.adding_sub = false;
        self.detail.adding_link = false;
        self.detail.error = None;
        self.draft_id = threadbear_core::notes::new_id();
        self.title.update(cx, |f, cx| f.set_value("", window, cx));
        self.notes.update(cx, |f, cx| f.set_value("", window, cx));
        window.focus(&self.focus, cx);
        crate::web_panels::keyboard_to_app(window);
    }
    fn open_editor(&mut self, task: &TaskRow, notes_first: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy || (self.editor_open && self.draft_dirty_editor(cx)) {
            self.error = Some("Save or discard your draft before opening another task.".into());
            cx.notify();
            return;
        }
        self.editor_open = true;
        self.selected_task = Some(task.id.clone());
        self.editing = Some(task.id.clone());
        self.editing_baseline = Some(task.clone());
        self.status = column_id(task).to_string();
        self.project = task.project.clone();
        self.title.update(cx, |f, cx| f.set_value(task.title.clone(), window, cx));
        self.notes.update(cx, |f, cx| f.set_value(task.notes.clone(), window, cx));
        // The page has the keys (J, K, the arrows, Esc) until you click into
        // the title or description; Edit Note goes straight to the description.
        if notes_first {
            self.notes.update(cx, |f, cx| f.focus(window, cx));
            crate::web_panels::keyboard_to_app(window);
        } else {
            window.focus(&self.focus, cx);
        }
        crate::web_panels::keyboard_to_app(window);
        self.detail_scroll.set_offset(point(px(0.), px(0.)));
        // The description grows to its text on the frame after it's first laid
        // out at its width, so draw it again shortly after opening.
        cx.spawn(async move |this, cx| {
            for ms in [60, 250] {
                cx.background_executor().timer(std::time::Duration::from_millis(ms)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() { break; }
            }
        }).detach();
        self.load_detail(&task.id.clone(), cx);
        cx.notify();
    }
    /// The task's page saves a title or description once Mike leaves it.
    fn autosave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor_open && self.editing_baseline.is_some() && !self.busy && !self.title.read(cx).value().trim().is_empty() && self.draft_dirty_editor(cx) {
            self.keep_open = true;
            self.save(window, cx);
        }
    }
    fn draft_dirty_editor(&self, cx: &App) -> bool {
        let title = self.title.read(cx).value().to_string();
        let notes = self.notes.read(cx).value().to_string();
        match self.editing_baseline.as_ref() {
            Some(row) => title != row.title || notes != row.notes || self.status != column_id(row) || self.project != row.project,
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
        // A sub-task shows under its task, not as a row of its own, unless its task is gone.
        let ids: HashSet<&str> = self.rows.iter().map(|r| r.id.as_str()).collect();
        let mut rows: Vec<TaskRow> = self.rows.iter()
            .filter(|r| r.parent_id.as_deref().is_none_or(|p| !ids.contains(p)))
            .filter(|r| self.filter == json!("all") || r.project.clone().unwrap_or(Value::Null) == self.filter).cloned().collect();
        rows.sort_by(|a, b| match self.prefs["sort"].as_str().unwrap_or("manual") {
            "title" => a.title.cmp(&b.title),
            "newest" => b.created_at.total_cmp(&a.created_at),
            "oldest" => a.created_at.total_cmp(&b.created_at),
            _ => a.position.total_cmp(&b.position).then(a.id.cmp(&b.id)),
        });
        rows
    }
    // Navigate in the same group/column order that is drawn, skipping folded sections.
    fn pane_scroll(&self, key: &str) -> ScrollHandle {
        self.scrolls.borrow_mut().entry(key.to_string()).or_insert_with(ScrollHandle::new).clone()
    }
    fn bounds_marker(&self, key: String) -> AnyElement {
        let bounds = self.task_bounds.clone();
        canvas(move |rect, _, _| { bounds.borrow_mut().insert(key.clone(), rect); }, |_, _, _, _| {})
            .absolute().inset_0().size_full().into_any_element()
    }
    fn reveal(&self, key: &str, scroll: &ScrollHandle, horizontal: bool) {
        let Some(bounds) = self.task_bounds.borrow().get(key).copied() else { return };
        let pane = scroll.bounds();
        let mut offset = scroll.offset();
        if horizontal {
            if bounds.right() > pane.right() { offset.x -= bounds.right() - pane.right(); }
            else if bounds.left() < pane.left() { offset.x += pane.left() - bounds.left(); }
        } else {
            if bounds.bottom() > pane.bottom() { offset.y -= bounds.bottom() - pane.bottom(); }
            else if bounds.top() < pane.top() { offset.y += pane.top() - bounds.top(); }
        }
        scroll.set_offset(offset);
    }
    /// The tasks in the order they're drawn, skipping folded sections.
    fn nav_order(&self) -> Vec<TaskRow> {
        let rows = self.visible();
        let board = self.view_name() == "board";
        let order = if board { self.board_order() } else { self.list_order() };
        let mut visible = Vec::new();
        for group in self.groups(&rows) {
            if self.collapsed.contains(&format!("project:{group}")) { continue; }
            for column in &order {
                if !board && ((!self.show_done && self.canonical(column) == "done") || self.collapsed.contains(&format!("status:{group}:{column}"))) { continue; }
                visible.extend(rows.iter().filter(|r| column_id(r) == column && (group == "all" || project_key(&r.project) == group)).cloned());
            }
        }
        visible
    }
    fn navigate_task(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let board = self.view_name() == "board";
        let visible = self.nav_order();
        if visible.is_empty() { return; }
        let current = visible.iter().position(|t| Some(&t.id) == self.selected_task.as_ref());
        if key == "enter" {
            if let Some(index) = current { self.open_editor(&visible[index], false, window, cx); }
        } else {
            let index = match (key, current) {
                ("up", Some(i)) => i.saturating_sub(1),
                ("up", None) => visible.len() - 1,
                (_, Some(i)) => (i + 1).min(visible.len() - 1),
                _ => 0,
            };
            let selected = &visible[index];
            self.selected_task = Some(selected.id.clone());
            if board {
                let group = if self.by_project() { project_key(&selected.project) } else { "all".into() };
                let column = format!("column:{group}:{}", column_id(selected));
                self.reveal(&column, &self.scroll, false);
                self.reveal(&column, &self.pane_scroll(&format!("board:{group}")), true);
                self.reveal(&selected.id, &self.pane_scroll(&column), false);
            } else { self.reveal(&selected.id, &self.scroll, false); }
            window.focus(&self.focus, cx);
            crate::web_panels::keyboard_to_app(window);
            cx.notify();
        }
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
        let peers: Vec<(String, f64)> = self.visible().into_iter().filter(|r| column_id(r) == status && r.id != drag.id && (group == "all" || project_key(&r.project) == group)).map(|r| (r.id, r.position)).collect();
        let mut patch = json!({"status": self.canonical(status),"columnId":status});
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
            self.set_pref(if list { "sectionOrder" } else { "boardColumnOrder" }, json!(next), cx);
        }
    }
    fn drop_project(&mut self, drag: &ProjectDrag, target: &str, cx: &mut Context<Self>) {
        let groups = self.groups(&self.visible());
        let next = task_order::move_to(&groups, &drag.0, target);
        if next != groups {
            self.set_pref("projectOrder", json!(next), cx);
        }
    }

    fn send_task(&mut self, row: &TaskRow, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = row.thread.as_ref().and_then(|v| v["id"].as_str()).map(String::from) {
            let app = self.app.clone();
            app.update(cx, |app, cx| { app.close_tasks(cx); app.open_thread_link(&id, window, cx); }).ok();
        } else if let Some(error) = &row.thread_error { self.error = Some(error.clone()); cx.notify(); }
        else if row.thread_command_id.is_none() && !self.busy { self.change("tasks.startThread", json!({"id":row.id}), cx); }
    }
    fn more_menu(&self, task: &TaskRow, cx: &mut Context<Self>) -> AnyElement {
        let page = cx.entity().downgrade();
        let t = task.clone();
        let projects = self.project_choices();
        let statuses = if self.view_name() == "list" { self.list_order() } else { self.board_order() };
        let labels: Vec<String> = statuses.iter().map(|s| self.column_name(s)).collect();
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
                let (p, row) = (page.clone(), t.clone());
                let label = if row.thread.is_some() { "View Thread" } else if row.thread_error.is_some() { "Thread Failed" } else if row.thread_command_id.is_some() { "Starting Thread…" } else { "Send to Thread" };
                menu = menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| { p.update(cx, |p, cx| p.send_task(&row, window, cx)).ok(); }));
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
                let (p, row) = (page.clone(), t.clone());
                menu = menu.submenu_with_icon(Some(priority_icon(Some("high"), 14., cx)), "Priority", window, cx, move |sub, _, cx| {
                    let mut sub = sub;
                    let mut choices: Vec<(Option<&'static str>, &'static str)> = vec![(None, "No priority")];
                    choices.extend(threadbear_core::task_dates::PRIORITIES.iter().map(|(id, name)| (Some(*id), *name)));
                    for (value, label) in choices {
                        let (p, id) = (p.clone(), row.id.clone());
                        sub = sub.item(PopupMenuItem::new(label).icon(priority_icon(value, 14., cx)).checked(row.priority.as_deref() == value).on_click(move |_, _, cx| {
                            p.update(cx, |p, cx| p.set_field(&id, "priority", value.map_or(Value::Null, |v| json!(v)), cx)).ok();
                        }));
                    }
                    sub
                });
                let (p, row) = (page.clone(), t.clone());
                menu = menu.submenu_with_icon(Some(icons::symbol("calendar")), "Due date", window, cx, move |sub, _, _| {
                    let mut sub = sub;
                    let picks = threadbear_core::task_dates::quick_picks(detail::today());
                    let current = row.due_date.clone();
                    for (label, day) in [("Today", picks[0].1), ("Tomorrow", picks[1].1), ("End of week", picks[2].1)] {
                        let (p, id, value) = (p.clone(), row.id.clone(), threadbear_core::task_dates::format(day));
                        sub = sub.item(PopupMenuItem::new(format!("{label}  {}", threadbear_core::task_dates::long_label(day))).checked(current.as_deref() == Some(value.as_str())).on_click(move |_, _, cx| {
                            p.update(cx, |p, cx| p.set_field(&id, "dueDate", json!(value), cx)).ok();
                        }));
                    }
                    let (p, id) = (p.clone(), row.id.clone());
                    sub.separator().item(PopupMenuItem::new("Clear").disabled(current.is_none()).on_click(move |_, _, cx| {
                        p.update(cx, |p, cx| p.set_field(&id, "dueDate", Value::Null, cx)).ok();
                    }))
                });
                menu = menu.separator();
                for (index, status) in statuses.iter().enumerate() {
                    let (p, id, status) = (page.clone(), t.id.clone(), status.clone());
                    let icon = status_icon(&status, cx);
                    menu = menu.item(PopupMenuItem::new(labels[index].clone()).icon(icon).checked(column_id(&t) == status).on_click(move |_, _, cx| {
                        p.update(cx, |p, cx| p.patch(&id, json!({"status": p.canonical(&status),"columnId":status}), None, cx)).ok();
                    }));
                }
                let (p, id) = (page.clone(), t.id.clone());
                menu.separator().item(PopupMenuItem::new("Delete").icon(icons::symbol("trash")).on_click(move |_, window, cx| {
                    p.update(cx, |p, cx| p.ask_delete(id.clone(), window, cx)).ok();
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
    /// A row's or card's due date and sub-task count, when it has them.
    fn meta_chips(&self, task: &TaskRow, cx: &App) -> Vec<AnyElement> {
        let mut out = Vec::new();
        if task.status != "done" {
            out.extend(task.due_date.as_deref().and_then(|d| due_chip(d, cx)));
        }
        let (done, all) = self.subtask_counts(&task.id);
        if all > 0 { out.push(subtask_chip(done, all, cx)); }
        if !task.links.is_empty() {
            out.push(div().id(SharedString::from(format!("links-mark-{}", task.id))).flex_none().tip(if task.links.len() == 1 { "1 link".to_string() } else { format!("{} links", task.links.len()) })
                .child(icons::symbol("link").with_size(px(12.)).text_color(style::tertiary_label(cx))).into_any_element());
        }
        out
    }
    fn priority_mark(&self, task: &TaskRow, cx: &App) -> Option<AnyElement> {
        let priority = task.priority.as_deref()?;
        let name = threadbear_core::task_dates::priority_name(Some(priority));
        Some(div().id(SharedString::from(format!("priority-mark-{}", task.id))).flex_none().tip(format!("{name} priority")).child(priority_icon(Some(priority), 14., cx)).into_any_element())
    }
    fn row(&self, task: &TaskRow, group: &str, cx: &mut Context<Self>) -> AnyElement {
        let id = task.id.clone();
        let next = if task.status == "done" { "not done" } else { "done" };
        let (done, accent, hover) = (task.status == "done", style::accent(cx), style::hover_fill(cx));
        let (drop_status, drop_group, drop_before) = (column_id(task).to_string(), group.to_string(), task.id.clone());
        let check_task = task.clone();
        let edit = task.clone();
        h_flex()
            .id(SharedString::from(format!("task-{id}")))
            .group("task-row")
            .relative().child(self.bounds_marker(id.clone()))
            .when(self.selected_task.as_deref() == Some(id.as_str()), |d| d.bg(style::accent(cx).opacity(0.12)))
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
            // A dead gap after the box, so a click meant for the title never ticks it.
            .child(
                div().pt(px(2.)).mr(px(10.)).flex_none().child(
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
                    .children(self.priority_mark(task, cx))
                    .child(div().min_w_0().text_size(px(14.)).when(done, |d| d.line_through().text_color(style::tertiary_label(cx))).child(task.title.clone().test_text(task.title.clone())))
                    .when(!task.notes.is_empty(), |d| d.child(icons::symbol("text.alignleft").with_size(px(12.)).text_color(style::tertiary_label(cx)))),
            )
            .children(self.meta_chips(task, cx))
            .children(self.chip(task, cx))
            .child(div().flex_none().invisible().group_hover("task-row", |s| s.visible()).child(self.more_menu(task, cx)))
            .into_any_element()
    }
    fn card(&self, task: &TaskRow, group: &str, cx: &mut Context<Self>) -> AnyElement {
        let id = task.id.clone();
        let (accent, edge) = (style::accent(cx), style::card_border(cx));
        let (drop_status, drop_group, drop_before) = (column_id(task).to_string(), group.to_string(), task.id.clone());
        let edit = task.clone();
        let done = task.status == "done";
        v_flex()
            .id(SharedString::from(format!("card-{id}")))
            .role(Role::Button)
            .aria_label(SharedString::from(format!("Open {}", task.title)))
            .group("task-card")
            .relative().child(self.bounds_marker(id.clone()))
            .gap(px(6.))
            .p(px(10.))
            .pl(px(12.))
            .mb(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(if self.selected_task.as_deref() == Some(id.as_str()) { accent } else { edge })
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
            .when(!task.notes.is_empty() || self.chip(task, cx).is_some() || task.priority.is_some() || !self.meta_chips(task, cx).is_empty(), |d| {
                d.child(
                    h_flex()
                        .gap(px(6.))
                        .flex_wrap()
                        .children(self.priority_mark(task, cx))
                        .children(self.meta_chips(task, cx))
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
        let items: Vec<&TaskRow> = rows.iter().filter(|r| column_id(r) == status && (group == "all" || project_key(&r.project) == group)).collect();
        if items.is_empty() && !dragging {
            return None;
        }
        let key = format!("status:{group}:{status}");
        let (accent, soft) = (style::accent(cx), style::accent(cx).opacity(0.08));
        let (s1, g1, s2) = (status.to_string(), group.to_string(), status.to_string());
        let label = h_flex().gap(px(8.)).child(status_icon(&self.canonical(status), cx)).child(self.column_name(status)).into_any_element();
        let drag_name = self.column_name(status);
        let header = h_flex()
            .id(SharedString::from(format!("section-{key}")))
            .h(px(32.))
            .px(px(8.))
            .rounded(px(6.))
            .bg(style::hover_fill(cx))
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .cursor_grab()
            .on_drag(StatusDrag(status.to_string()), move |_, _, _, cx| cx.new(|_| Ghost(drag_name.clone())))
            .drag_over::<StatusDrag>(move |s, _, _, _| s.border_t_2().border_color(accent))
            .on_drop(cx.listener(move |this, drag: &StatusDrag, _, cx| this.drop_status(drag, &s2, true, cx)))
            .child(self.fold(key.clone(), self.column_name(status), label, items.len(), cx));
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
                .child(div().flex_1().child(Textarea::new(&self.composer).appearance(false).disabled(self.busy)))
                .child(Button::new("tasks-add").icon(icons::symbol("arrow.up")).small().primary().loading(self.busy).disabled(self.busy).tooltip("Add task (Return)").on_click(cx.listener(|this, _, window, cx| this.submit_composer(window, cx)))),
        );
        if rows.is_empty() && !self.loading {
            return out.child(v_flex().items_center().gap(px(10.)).py(px(64.)).text_color(style::tertiary_label(cx)).child(status_icon("todo", cx)).child("Nothing here yet. Type a task above and press Return.")).into_any_element();
        }
        let statuses: Vec<String> = self.list_order().into_iter().filter(|s| self.canonical(s) != "done" || self.show_done).collect();
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
        // Wrapped, every column shows on rows of their own and the page scrolls
        // down; otherwise one row that scrolls sideways.
        let wrap = self.wrap_columns;
        let mut columns = h_flex().id(SharedString::from(format!("columns-{group}"))).items_start().w_full().min_w_0().max_w_full().when(group == "all" && !wrap, |d| d.h_full()).gap(px(12.)).pb(px(8.))
            .when(wrap, |d| d.flex_wrap())
            .when(!wrap, |d| d.overflow_x_scroll().track_scroll(&self.pane_scroll(&format!("board:{group}"))));
        for status in self.board_order() {
            let items: Vec<&TaskRow> = rows.iter().filter(|r| column_id(r) == status && (group == "all" || project_key(&r.project) == group)).collect();
            let (accent, soft) = (style::accent(cx), style::accent(cx).opacity(0.08));
            let (s1, g1, s2, s3, g3) = (status.clone(), group.to_string(), status.clone(), status.clone(), group.to_string());
            let adding = self.adding_in.as_ref() == Some(&(group.to_string(), status.clone()));
            let drag_name = self.column_name(&status);
            let header = h_flex()
                .id(SharedString::from(format!("column-head-{group}-{status}")))
                .h(px(34.))
                .px(px(6.))
                .gap(px(8.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .cursor_grab()
                .tip("Drag to reorder columns")
                .on_drag(StatusDrag(status.clone()), move |_, _, _, cx| cx.new(|_| Ghost(drag_name.clone())))
                .drag_over::<StatusDrag>(move |s, _, _, _| s.border_l_2().border_color(accent))
                .on_drop(cx.listener(move |this, drag: &StatusDrag, _, cx| this.drop_status(drag, &s2, false, cx)))
                .child(status_icon(&self.canonical(&status), cx))
                .child(Button::new(SharedString::from(format!("rename-{group}-{status}"))).label(self.column_name(&status)).ghost().small().tooltip("Rename column").on_click({ let id = status.clone(); cx.listener(move |this, _, window, cx| this.edit_column(Some(id.clone()), window, cx)) }))
                .child(div().text_color(style::tertiary_label(cx)).child(items.len().to_string()))
                .child(div().flex_1())
                .child(
                    Button::new(SharedString::from(format!("add-{group}-{status}")))
                        .icon(icons::symbol("plus"))
                        .ghost()
                        .xsmall()
                        .tooltip(format!("Add a task to {}", self.column_name(&status)))
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
            let column_key = format!("column:{group}:{status}");
            let mut column = v_flex()
                .id(SharedString::from(format!("column-{group}-{status}")))
                .relative().child(self.bounds_marker(column_key.clone()))
                .flex_none()
                .w(px(280.))
                .min_h(px(160.))
                .p(px(6.))
                .rounded(px(10.))
                .bg(style::hover_fill(cx))
                .drag_over::<TaskDrag>(move |s, _, _, _| s.bg(soft))
                .on_drop(cx.listener(move |this, drag: &TaskDrag, _, cx| this.drop_task(drag, &s1, &g1, None, cx)))
                .child(header);
            let mut cards = v_flex().id(SharedString::from(format!("cards-{group}-{status}"))).min_h_0()
                .when(!wrap, |d| d.max_h(px((self.viewport_height - 210.).max(140.))).overflow_y_scroll().track_scroll(&self.pane_scroll(&column_key)));
            if adding {
                cards = cards.child(div().mb(px(6.)).px(px(10.)).py(px(8.)).min_h(px(64.)).rounded(px(8.)).border_1().border_color(style::accent(cx)).bg(cx.theme().background).text_size(px(14.))
                    .child(Textarea::new(&self.column_input).appearance(false).disabled(self.busy)));
            }
            for task in &items {
                cards = cards.child(self.card(task, group, cx));
            }
            column = column.child(cards);
            if !adding {
                let (group, status) = (group.to_string(), status.clone());
                column = column.child(Button::new(SharedString::from(format!("empty-add-{group}-{status}")))
                    .label("Add a task").ghost().small().w_full().min_h(px(if items.is_empty() { 100. } else { 28. })).tooltip("Add a task in this column").disabled(self.busy)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.adding_in = Some((group.clone(), status.clone()));
                        this.column_input.update(cx, |input, cx| { input.set_value("", window, cx); input.focus(window, cx); });
                        crate::web_panels::keyboard_to_app(window);
                        cx.notify();
                    })));
            }
            columns = columns.child(column);
        }
        columns.child(Button::new(SharedString::from(format!("new-column-{group}"))).label("Add column").small().ghost().tooltip("Add a board column").disabled(self.busy || self.definitions().len() >= 32).on_click(cx.listener(|this, _, window, cx| this.edit_column(None, window, cx)))).into_any_element()
    }
    fn board_view(&self, rows: &[TaskRow], cx: &mut Context<Self>) -> AnyElement {
        if !self.by_project() {
            return div().size_full().min_w_0().child(self.board_columns("all", rows, cx)).into_any_element();
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
        let (prefs, filter, view, show_done, wrap) = (self.prefs.clone(), self.filter.clone(), self.view_name(), self.show_done, self.wrap_columns);
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
                if view == "board" {
                    let p = page.clone();
                    menu = menu.item(PopupMenuItem::new("Wrap Columns").checked(wrap).on_click(move |_, _, cx| {
                        p.update(cx, |p, cx| p.set_wrap(!p.wrap_columns, cx)).ok();
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
    /// Wrap Columns: kept on this Mac, beside the app's other display choices.
    fn set_wrap(&mut self, on: bool, cx: &mut Context<Self>) {
        self.wrap_columns = on;
        self.app.update(cx, |app, _| {
            app.new_thread.prefs.tasks_wrap_columns = on;
            crate::prefs::save(&app.new_thread.prefs);
        }).ok();
        cx.notify();
    }
    fn upload_photo(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        if self.busy { return; }
        self.busy = true;
        self.error = None;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            // Read in the process granted access by the picker, off the UI thread.
            let image = cx.background_executor().spawn(async move {
                let read = threadbear_core::local_files::read(&json!({"path":path,"maxBytes":10*1024*1024})).map_err(|e| format!("Could not read that photo: {e}"))?;
                if read["truncated"] == true { return Err("Choose a photo under 10 MB.".to_string()); }
                Ok(read["data"].clone())
            }).await;
            let result = match image {
                Ok(base64) => call(client, "tasks.photoData", json!({"base64":base64})).await.map_err(|e| e.message),
                Err(error) => Err(error),
            };
            this.update(cx, |this, cx| {
                this.busy = false;
                match result { Ok(_) => this.refresh(cx), Err(error) => this.error = Some(error) }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn choose_photo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: false, prompt: Some("Choose".into()) });
        cx.spawn_in(window, async move |this, cx| {
            match paths.await {
                Ok(Ok(Some(paths))) => if let Some(path) = paths.first() {
                    this.update(cx, |this, cx| this.upload_photo(path.clone(), cx)).ok();
                },
                Ok(Ok(None)) => {},
                _ => { this.update(cx, |this, cx| { this.error = Some("Could not open the photo picker. Try again, or drop a photo here.".into()); cx.notify(); }).ok(); }
            }
        }).detach();
    }
    fn customize_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut swatches = h_flex().gap(px(6.));
        for (name, appearance) in [("Light", style::Appearance::Light), ("Dark", style::Appearance::Dark)] {
            swatches = swatches.child(Button::new(SharedString::from(format!("task-theme-{name}"))).label(name).small().outline().tooltip(format!("Use {name} appearance in Tasks and the app")).on_click(cx.listener(move |this, _, window, cx| {
                this.app.update(cx, |app, cx| app.set_appearance(appearance, window, cx)).ok();
                this.change("tasks.setPreferences", json!({"theme":appearance.name(),"backgroundColor":""}), cx);
            })));
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
                    this.upload_photo(path.clone(), cx);
                }
            }))
            .child(div().text_size(px(12.)).text_color(style::secondary_label(cx)).child("Appearance"))
            .child(swatches)
            .when_some(self.error.clone(), |d, error| d.child(div().text_size(px(12.)).text_color(style::failed_text(cx)).child(error)))
            .child(div().text_size(px(12.)).text_color(style::secondary_label(cx)).child("Photo"))
            .child(
                h_flex()
                    .gap(px(6.))
                    .child(div().flex_1().child(Button::new("tasks-photo").label(if has_photo { "Change Photo…" } else { "Choose Photo…" }).loading(self.busy).small().outline().w_full().tooltip("A PNG, JPEG, WebP or GIF under 10 MB. You can drop one here too.").disabled(self.busy).on_click(cx.listener(|this, _, window, cx| this.choose_photo(window, cx)))))
                    .when(has_photo, |d| d.child(Button::new("tasks-reset-background").label("Remove").small().outline().tooltip("Remove the photo").disabled(self.busy).on_click(cx.listener(|this, _, _, cx| this.set_pref("backgroundPhoto", Value::Null, cx))))),
            );
        if has_photo {
            let fade = self.prefs["fade"].as_f64().unwrap_or(0.85) as f32;
            let place = place_value(self.prefs["photoPosition"].as_str().unwrap_or("center"));
            // Another Mac or the web changed it: the sliders follow, unless one is being dragged.
            for (state, target) in [(self.fade_slider.clone(), fade), (self.place_slider.clone(), place)] {
                if (state.read(cx).value().start() - target).abs() > 0.005 && !cx.has_active_drag() {
                    cx.defer(move |cx| {
                        if let Some(window) = cx.active_window() {
                            window.update(cx, |_, window, cx| state.update(cx, |s, cx| s.set_value(target, window, cx))).ok();
                        }
                    });
                }
            }
            let slider_row = |label: &'static str, id: &'static str, tip: &'static str, state: &Entity<SliderState>, value: String, cx: &mut Context<Self>| {
                h_flex()
                    .gap(px(10.))
                    .child(div().w(px(56.)).flex_none().text_size(px(12.)).text_color(style::secondary_label(cx)).child(label))
                    .child(div().id(id).test_support().a11y_role(Role::GenericContainer).aria_label(tip).flex_1().min_w_0().tip(tip).child(Slider::new(state).horizontal().accessibility_label(tip)))
                    .child(div().w(px(48.)).flex_none().text_right().text_size(px(12.)).text_color(style::tertiary_label(cx)).child(value))
            };
            let place_label = match place_name(place) { "top" => "Top", "bottom" => "Bottom", _ => "Center" };
            panel = panel
                .child(slider_row("Fade", "tasks-fade", "How much the photo fades behind your tasks", &self.fade_slider, format!("{}%", (fade * 100.).round() as i32), cx))
                .child(slider_row("Position", "tasks-place", "Which part of the photo shows: top, center or bottom", &self.place_slider, place_label.to_string(), cx));
        }
        let default_view = self.prefs["defaultView"].as_str().unwrap_or("list").to_string();
        let mut views = h_flex().self_start().gap(px(2.)).p(px(2.)).rounded(px(7.)).bg(style::hover_fill(cx));
        for (name, label) in [("list", "List"), ("board", "Board")] {
            views = views.child(Button::new(SharedString::from(format!("tasks-default-{name}"))).label(label).accessibility_label(format!("Open Tasks in {label}")).small().ghost().selected(default_view == name)
                .tooltip(format!("Open Tasks in the {} every time", label.to_lowercase()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_pref("defaultView", json!(name), cx);
                    this.view = Some(name.into());
                    cx.notify();
                })));
        }
        panel = panel.child(div().h(px(1.)).bg(style::hairline(cx)))
            .child(div().text_size(px(12.)).text_color(style::secondary_label(cx)).child("Open Tasks in"))
            .child(views);
        panel.into_any_element()
    }
    fn editor_sheet(&self, cx: &mut Context<Self>) -> AnyElement {
        let page = cx.entity().downgrade();
        let status = self.status.clone();
        let statuses = self.board_order();
        let labels: Vec<String> = statuses.iter().map(|s| self.column_name(s)).collect();
        let status_button = Button::new("task-status").label(self.column_name(&status)).icon(status_icon(&status, cx)).small().outline().tooltip("Status").dropdown_menu(move |menu: PopupMenu, _, cx| {
            let mut menu = menu;
            for (index, s) in statuses.iter().enumerate() {
                let (p, s) = (page.clone(), s.clone());
                menu = menu.item(PopupMenuItem::new(labels[index].clone()).icon(status_icon(&s, cx)).checked(status == s).on_click(move |_, _, cx| {
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
        let linked = self.editing.as_ref().and_then(|id| self.rows.iter().find(|r| &r.id == id));
        let send_label = match linked {
            Some(row) if row.thread.is_some() => "View Thread",
            Some(row) if row.thread_error.is_some() => "Thread Failed",
            Some(row) if row.thread_command_id.is_some() => "Starting Thread…",
            _ => "Send to Thread",
        };
        let starting = linked.is_some_and(|r| r.thread_command_id.is_some() && r.thread.is_none() && r.thread_error.is_none());
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
                        this.ask_delete(id.clone(), window, cx);
                    }))))
                    .child(div().flex_1())
                    .when_some(self.error.clone(), |d, e| d.child(div().text_size(px(12.)).text_color(style::failed_text(cx)).child(e.clone().test_text(e))))
                    .child(Button::new("task-send-thread").label(send_label).small().outline().tooltip("Save this task and start a thread with its title, notes and project").disabled(self.busy || starting).loading(starting).on_click(cx.listener(|this, _, window, cx| {
                        if let Some(row) = this.editing.as_ref().and_then(|id| this.rows.iter().find(|r| &r.id == id)).cloned() {
                            if row.thread.is_some() || row.thread_command_id.is_some() { this.send_task(&row, window, cx); return; }
                        }
                        this.send_after_save = true; this.save(window, cx);
                    })))
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
        self.viewport_height = f32::from(window.viewport_size().height) / style::zoomed(1.);
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
        // An icon, as Linear has it; the dot says a photo is set.
        let customize = Button::new("tasks-customize").icon(icons::symbol("slider.horizontal.3")).accessibility_label("Customize Tasks").small().outline().selected(self.customize)
            .tooltip("Appearance, background photo and the default view")
            .on_click(cx.listener(|this, _, _, cx| {
                this.customize = !this.customize;
                cx.notify();
            }));
        let dot = style::accent(cx);
        actions = actions.child(div().relative().child(customize).when(self.photo.is_some(), |d| d.child(div().absolute().top(px(-2.)).right(px(-2.)).size(px(7.)).rounded_full().bg(dot))));
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

        // An existing task opens on a page of its own in place of the list or
        // board (task_detail.rs); a new one is still the small sheet.
        let open_row = self.editor_open.then(|| self.editing_baseline.as_ref().and_then(|_| self.open_row().cloned())).flatten();
        let mut page = v_flex()
            .id("tasks-content")
            .size_full()
            .px(px(28.))
            .pt(px(24.))
            .text_color(style::label_text(cx));
        if let Some(row) = &open_row {
            page = page.child(self.detail_view(row, window, cx));
        } else {
            page = page.child(header);
        }
        if let Some(error) = self.error.clone().filter(|_| !self.editor_open) {
            page = page.child(div().mb(px(10.)).text_size(px(13.)).text_color(style::failed_text(cx)).child(error.clone().test_text(error)));
        }
        let body = if open_row.is_some() { div().into_any_element() } else if view == "list" { self.list_view(&rows, cx) } else { self.board_view(&rows, cx) };
        // One board fills the pane itself, so a sideways swipe anywhere under the
        // columns scrolls them, not only over a column (the page's vertical
        // scroller below would take it and do nothing).
        if open_row.is_some() {
        } else if view == "board" && !self.by_project() && !self.wrap_columns {
            page = page.child(div().id("tasks-board").flex().w_full().min_w_0().flex_1().min_h_0().child(body));
        } else {
            page = page.child(div().id("tasks-scroll").w_full().min_w_0().flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.scroll).pb(px(80.)).child(body));
        }

        let bg = cx.theme().background;
        // The page, the toast and the editor sheet are all read by the scripted control.
        let mut shell = div().id("tasks-page").test_support().relative().size_full().overflow_hidden().bg(bg)
            .key_context("TasksPage")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let modifiers = &event.keystroke.modifiers;
                if modifiers.platform || modifiers.control || modifiers.alt || modifiers.shift { return; }
                let key = event.keystroke.key.as_str();
                // The task's page: J or the down arrow for the next task, K or up for the one before.
                if this.editor_open && this.editing_baseline.is_some() && this.focus.is_focused(window) && !this.busy {
                    match key {
                        "down" | "j" => { this.step_task(1, window, cx); cx.stop_propagation(); }
                        "up" | "k" => { this.step_task(-1, window, cx); cx.stop_propagation(); }
                        _ => {}
                    }
                    return;
                }
                if this.editor_open || this.busy { return; }
                let empty_composer = this.composer.read(cx).value().is_empty() && this.composer.read(cx).focus_handle(cx).is_focused(window);
                if (this.focus.is_focused(window) || (empty_composer && key == "down")) && matches!(key, "up" | "down" | "enter") {
                    this.navigate_task(key, window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_action(cx.listener(|this, _: &CloseTasks, window, cx| {
                if this.detail.picker.is_some() {
                    this.detail.picker = None;
                    cx.notify();
                    return;
                }
                if this.editor_open {
                    this.leave_detail(window, cx);
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
            }));
        if self.photo.is_some() {
            if let Some((image, size)) = &self.photo_image {
                let y = match self.prefs["photoPosition"].as_str() {
                    Some("top") => 0.,
                    Some("bottom") => 1.,
                    _ => 0.5,
                };
                let (x, y, w, h) = threadbear_core::looks::BackdropFocus::new(0.5, y).frame(*size, self.background_size);
                shell = shell.child(img(ImageSource::Image(image.clone())).id("tasks-background-photo").absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).object_fit(ObjectFit::Fill).opacity((1. - self.prefs["fade"].as_f64().unwrap_or(0.85)) as f32));
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
        if self.editor_open && open_row.is_none() {
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
        if let Some(path) = cmd["photoPath"].as_str() { p.upload_photo(path.into(), cx); }
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
        if let Some(on) = cmd["wrap"].as_bool() {
            p.set_wrap(on, cx);
        }
        // The task's page: `task` names one (title or id), else the open one.
        let target = cmd["task"].as_str().and_then(|t| p.rows.iter().find(|r| r.title == t || r.id == t).map(|r| r.id.clone())).or_else(|| p.editing.clone());
        if let Some(id) = target.clone() {
            if cmd.get("priority").is_some() {
                p.set_field(&id, "priority", cmd["priority"].clone(), cx);
            }
            if let Some(due) = cmd["due"].as_str() {
                let today = detail::today();
                let value = match due {
                    "clear" => Value::Null,
                    "today" => json!(threadbear_core::task_dates::format(today)),
                    "tomorrow" => json!(threadbear_core::task_dates::format(today + threadbear_core::chrono::Duration::days(1))),
                    "overdue" => json!(threadbear_core::task_dates::format(today - threadbear_core::chrono::Duration::days(2))),
                    day => json!(day),
                };
                p.set_field(&id, "dueDate", value, cx);
            }
        }
        if p.editor_open && p.editing_baseline.is_some() {
            if let Some(name) = cmd["picker"].as_str() {
                p.detail.picker = match name { "priority" => Some("priority"), "due" => Some("due"), _ => None };
            }
            if let Some(by) = cmd["month"].as_i64() {
                let (y, m) = p.detail.month;
                p.detail.month = threadbear_core::task_dates::shift_month(y, m, by as i32);
            }
            if let Some(which) = cmd["activity"].as_str() {
                p.detail.comments_only = which == "comments";
            }
            if let Some(text) = cmd["comment"].as_str() {
                p.comment_input.update(cx, |f, cx| f.set_value(text.to_owned(), window, cx));
                p.send_comment(window, cx);
            }
            if let Some(title) = cmd["subtask"].as_str() {
                p.detail.adding_sub = true;
                p.sub_input.update(cx, |f, cx| f.set_value(title.to_owned(), window, cx));
                p.add_subtask(window, cx);
            }
            if let Some(url) = cmd["addLink"].as_str() {
                p.link_url.update(cx, |f, cx| f.set_value(url.to_owned(), window, cx));
                p.link_title.update(cx, |f, cx| f.set_value(cmd["linkTitle"].as_str().unwrap_or("").to_owned(), window, cx));
                p.add_link(window, cx);
            }
            if let Some(id) = cmd["attachPrint"].as_str() {
                let title = cmd["printTitle"].as_str().unwrap_or(id).to_string();
                p.attach_print(id, &title, cx);
            }
            if let Some(path) = cmd["imagePath"].as_str() {
                p.attach_image(path.into(), cx);
            }
            if let Some(by) = cmd["step"].as_i64() {
                p.step_task(by as i32, window, cx);
            }
            if let Some(y) = cmd["detailScroll"].as_f64() {
                p.detail_scroll.set_offset(point(px(0.), px(-y as f32)));
            }
            if cmd["back"].as_bool() == Some(true) {
                p.leave_detail(window, cx);
            }
        }
        if let Some(on) = cmd["showDone"].as_bool() {
            p.show_done = on;
        }
        cx.notify();
    });
    Ok(json!("ok"))
}
