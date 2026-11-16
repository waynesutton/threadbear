// Cloud-owned Tasks. The page retains drafts and shows failed writes instead
// of pretending a disconnected edit has been saved.
use crate::{app::Threadbear, call, style::{self, px}};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Disableable as _, Selectable as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::base::{TestSupportExt as _, TestTextExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};

actions!(tasks_page, [CloseTasks]);

#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskRow {
    id: String, title: String, notes: String, status: String,
    project: Option<Value>, position: f64, created_at: f64,
}
#[derive(Clone)]
struct TaskDrag { id: String, title: String }
#[derive(Clone)]
struct ColumnDrag(String);
struct TaskGhost(String);
impl Render for TaskGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().p(px(12.)).rounded(px(6.)).bg(cx.theme().background).child(self.0.clone())
    }
}

impl Threadbear {
    pub(crate) fn toggle_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.close_tasks(cx) || !self.experiment_on("tasks") { return; }
        self.close_settings(cx); self.close_notes(cx); self.close_skills(cx);
        let app = cx.entity().downgrade();
        let client = self.client.clone();
        let page = self.tasks_draft_page.take().unwrap_or_else(|| cx.new(|cx| TasksPage::new(app, client, window, cx)));
        let focus = page.read(cx).focus.clone();
        window.focus(&focus,cx);
        crate::web_panels::keyboard_to_app(window);
        page.update(cx, |p, cx| {
            p.refresh(cx);
            p.change("tasks.setPreferences", json!({"theme":style::appearance().name()}), cx);
        });
        self.tasks_page = Some(page); self.sync_menus(cx); cx.notify();
    }
    pub(crate) fn close_tasks(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(page) = self.tasks_page.take() else { return false; };
        if page.read(cx).draft_dirty(cx) || page.read(cx).busy { self.tasks_draft_page = Some(page); }
        let (window, focus) = (self.main_window, self.root_focus.clone());
        cx.defer(move |cx| { window.update(cx, |_, window, cx| { window.focus(&focus, cx); crate::web_panels::keyboard_to_app(window); }).ok(); });
        self.sync_menus(cx); cx.notify(); true
    }
    pub(crate) fn load_project_task_count(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.experiment_on("tasks") || self.task_counts.contains_key(id) { return; }
        let id = id.to_string(); self.task_counts.insert(id.clone(), None);
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "tasks.projectCount", json!({"project": id})).await;
            this.update(cx, |this, cx| {
                if let Ok(value) = result { this.task_counts.insert(id, Some(value["count"].as_u64().unwrap_or(0) as usize)); }
                cx.notify();
            }).ok();
        }).detach();
    }
    pub(crate) fn open_project_tasks(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.close_tasks(cx); self.toggle_tasks(window, cx);
        if let Some(page) = &self.tasks_page { page.update(cx, |p, cx| { p.project_filter_id = Some(id); cx.notify(); }); }
    }
    pub(crate) fn tasks_updated(&mut self, cx: &mut Context<Self>) {
        self.task_counts.clear();
        if let Some(page) = &self.tasks_page { page.update(cx, |p, cx| p.refresh(cx)); }
    }
    pub(crate) fn tasks_have_draft(&self, cx: &App) -> bool {
        self.tasks_page.as_ref().or(self.tasks_draft_page.as_ref()).is_some_and(|p| p.read(cx).busy || p.read(cx).draft_dirty(cx))
    }
    pub(crate) fn tasks_dump(&self, cx: &App) -> Value {
        let page = self.tasks_page.as_ref().map(|p| p.read(cx));
        json!({"open":page.is_some(), "loaded":page.is_some_and(|p| !p.loading && !p.busy && p.prefs["defaultView"].is_string()),
            "view":page.map(|p| p.view.as_deref().or(p.prefs["defaultView"].as_str()).unwrap_or("board")),
            "defaultView":page.map(|p| p.prefs["defaultView"].clone()),
            "tasks":page.map(|p| p.rows.iter().map(|t| json!({"id":t.id,"title":t.title,"status":t.status})).collect::<Vec<_>>()),
            "error":page.and_then(|p| p.error.clone())})
    }
    pub(crate) fn render_tasks_page(&self) -> AnyElement {
        match &self.tasks_page { Some(page) => div().size_full().child(page.clone()).into_any_element(), None => div().into_any_element() }
    }
}

pub(crate) struct TasksPage {
    app: WeakEntity<Threadbear>, client: std::sync::Arc<threadbear_client::runner::Client>, rows: Vec<TaskRow>, prefs: Value,
    title: Entity<InputState>, notes: Entity<TextareaState>, color: Entity<InputState>,
    projects: Vec<Value>, filter: Value, photo: Option<String>, customize: bool, refresh_again: bool,
    draft_id: String, project_filter_id: Option<String>, focus: FocusHandle,
    editing: Option<String>, editing_baseline: Option<TaskRow>, status: String, project: Option<Value>,
    editor_open: bool, view: Option<String>, busy: bool, loading: bool, error: Option<String>,
    undo: Option<String>, scroll: ScrollHandle, background_size: (f32,f32),
}
impl TasksPage {
    fn new(app: WeakEntity<Threadbear>, client: std::sync::Arc<threadbear_client::runner::Client>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        static BOUND: std::sync::Once = std::sync::Once::new();
        BOUND.call_once(|| cx.bind_keys([KeyBinding::new("escape", CloseTasks, Some("TasksPage"))]));
        let focus = cx.focus_handle(); window.focus(&focus,cx); crate::web_panels::keyboard_to_app(window);
        Self { app, client, rows: vec![], prefs: json!({}), title: cx.new(|cx| InputState::new(window, cx)), notes: cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, 6)), color: cx.new(|cx| InputState::new(window, cx)), projects: vec![], filter: json!("all"), photo: None, customize: false, refresh_again: false, draft_id: threadbear_core::notes::new_id(), project_filter_id: None, focus, editing: None, editing_baseline: None, status: "todo".into(), project: None, editor_open: false, view: None, busy: false, loading: false, error: None, undo: None, scroll: ScrollHandle::new(), background_size: (0.,0.) }
    }
    fn draft_dirty(&self, cx: &App) -> bool {
        let title = self.title.read(cx).value().to_string();
        let notes = self.notes.read(cx).value().to_string();
        match self.editing_baseline.as_ref() {
            Some(row) => title != row.title || notes != row.notes || self.status != row.status || self.project != row.project,
            None => !title.is_empty() || !notes.is_empty(),
        }
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.loading { self.refresh_again = true; return; }
        let client = self.client.clone();
        self.loading = true;
        cx.spawn(async move |this, cx| {
            let result = async {
                let prefs = call(client.clone(), "tasks.preferences", json!({})).await?;
                let projects = call(client.clone(), "tasks.projects", json!({})).await?;
                let photo = call(client.clone(), "tasks.photo", json!({})).await?;
                let mut rows = vec![]; let mut after = Value::Null;
                loop {
                    let mut args = json!({"limit": 250});
                    if !after.is_null() { args["after"] = after; }
                    let page = call(client.clone(), "tasks.list", args).await?;
                    if let Some(items) = page["items"].as_array() { rows.extend(items.iter().cloned()); }
                    after = page["next"].clone(); if after.is_null() { break; }
                }
                Ok::<_, threadbear_client::runner::RpcError>((prefs, rows, projects, photo))
            }.await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok((prefs, rows, projects, photo)) => { this.prefs = prefs;
                        this.projects = projects.as_array().cloned().unwrap_or_default();
                        this.photo = photo.as_str().map(String::from);
                        if let Some(id) = this.project_filter_id.take() {
                            if let Some(p) = this.projects.iter().find(|p| p["id"] == id) {
                                this.filter = json!({"runnerId":p["runnerId"],"id":p["id"]});
                                this.project = Some(this.filter.clone());
                            }
                        }
                        this.rows = rows.into_iter().filter_map(|r| serde_json::from_value(r).ok()).collect(); this.error = None; }
                    Err(e) => this.error = Some(e.message),
                }
                if this.refresh_again { this.refresh_again = false; this.refresh(cx); }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn change(&mut self, method: &'static str, args: Value, cx: &mut Context<Self>) {
        if self.busy { return; }
        let client = self.client.clone();
        self.busy = true; self.error = None; cx.notify();
        let deleted_id = (method == "tasks.remove").then(|| args["id"].as_str().unwrap_or("").to_string());
        cx.spawn(async move |this, cx| {
            let result = call(client, method, args).await;
            this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(_) => { if let Some(id) = deleted_id { this.undo = Some(id); } if method == "tasks.restore" { this.undo = None; } this.refresh(cx); },
                    Err(e) => this.error = Some(e.message)
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.title.read(cx).value().to_string(); if title.trim().is_empty() { return; }
        let args = json!({"title": title, "notes": self.notes.read(cx).value().to_string(), "status": self.status, "project": self.project});
        let method = if self.editing_baseline.is_some() { "tasks.update" } else { "tasks.create" };
        let mut args = args;
        if let Some(base) = &self.editing_baseline {
            let original = json!({"title":base.title,"notes":base.notes,"status":base.status,"project":base.project});
            args.as_object_mut().unwrap().retain(|key,value| original[key] != *value);
        }
 args["id"] = json!(self.editing.as_ref().unwrap_or(&self.draft_id));
        self.editing = Some(args["id"].as_str().unwrap().to_string());
        if self.busy { return; }
        let client = self.client.clone();
        self.busy = true; self.error = None; cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, method, args).await;
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                match result {
                    Ok(_) => {
                        this.editor_open = false;
                        this.editing = None; this.editing_baseline = None; this.draft_id = threadbear_core::notes::new_id();
                        this.title.update(cx, |f,cx| f.set_value("",window,cx));
                        this.notes.update(cx, |f,cx| f.set_value("",window,cx));
                        this.refresh(cx);
                    }
                    Err(e) => this.error = Some(e.message),
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn picker(&self, id: &'static str, label: String, choices: Vec<(String, Value)>, field: &'static str, cx: &mut Context<Self>) -> AnyElement {
        let page = cx.entity().downgrade();
        let selected = match field { "project" => self.project.clone().unwrap_or(Value::Null), "filter" => self.filter.clone(), "status" => json!(self.status), _ => self.prefs[field].clone() };
        Button::new(id).label(label).tooltip("Choose a task option").disabled(self.busy).dropdown_menu(move |menu: PopupMenu, _, _| {
            let mut menu = menu;
            for (label, value) in &choices {
                let page = page.clone(); let value = value.clone();
                menu = menu.item(PopupMenuItem::new(label.clone()).checked(value == selected).on_click(move |_, _, cx| {
                    page.update(cx, |this, cx| {
                        match field {
                            "project" => this.project = (!value.is_null()).then(|| value.clone()),
                            "filter" => { this.filter = value.clone(); this.project_filter_id = None; },
                            "status" => this.status = value.as_str().unwrap_or("todo").into(),
                            _ => this.change("tasks.setPreferences", json!({field: value}), cx),
                        }
                        cx.notify();
                    }).ok();
                }));
            }
            menu
        }).into_any_element()
    }
    fn project_choices(&self) -> Vec<(String, Value)> {
        let mut out = vec![("No project".into(), Value::Null)];
        out.extend(self.projects.iter().map(|p| (p["title"].as_str().unwrap_or("Project").to_string(), json!({"runnerId":p["runnerId"],"id":p["id"]})))); out
    }
    fn project_name(&self, value: &Value) -> String {
        self.projects.iter().find(|p| p["id"] == value["id"] && p["runnerId"] == value["runnerId"]).and_then(|p| p["title"].as_str()).unwrap_or("No project").to_string()
    }
    fn card(&self, task: &TaskRow, cx: &mut Context<Self>) -> AnyElement {
        let is_list = self.view.as_deref().or(self.prefs["defaultView"].as_str()) == Some("list");
        let id = task.id.clone(); let edit = task.clone(); let move_id = id.clone(); let delete_id = id.clone();
        let page = cx.entity().downgrade(); let status_id = id.clone();
        let status_menu = Button::new(SharedString::from(format!("status-{id}"))).label(task.status.clone()).tooltip("Move task to another status").disabled(self.busy)
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for status in ["backlog", "todo", "done"] {
                    let page = page.clone(); let id = status_id.clone();
                    menu = menu.item(PopupMenuItem::new(status).on_click(move |_,_,cx| { page.update(cx, |p,cx| p.change("tasks.update",json!({"id":id,"status":status}),cx)).ok(); }));
                } menu
            });
        let next = if task.status == "done" { "todo" } else { "done" };
        let previous = self.rows.iter().filter(|r| r.status == task.status && r.project == task.project && r.position < task.position).map(|r| r.position).max_by(f64::total_cmp);
        let target_project = task.project.clone(); let target_id = task.id.clone();
        let grouped = self.prefs["grouping"] == "project";
        let position = previous.map_or(task.position - 1000., |p| (p + task.position) / 2.); let status = task.status.clone();
        div().id(SharedString::from(format!("task-{id}"))).p(px(12.)).mb(px(8.)).rounded(px(7.)).border_1().border_color(cx.theme().border).bg(cx.theme().background)
            .when(!self.busy && self.prefs["sort"].as_str().unwrap_or("manual") == "manual", |d| d.on_drag(TaskDrag { id: id.clone(), title: task.title.clone() }, |drag, _, _, cx| cx.new(|_| TaskGhost(drag.title.clone()))))
            .on_drop(cx.listener(move |this, drag: &TaskDrag, _, cx| {
                if drag.id == target_id { return; }
                let mut args = json!({"id": drag.id, "status": status, "position": position});
                if grouped { args["project"] = json!(target_project); }
                this.change("tasks.update", args, cx);
            }))
            .child(div().flex().gap(px(8.)).when(!is_list, |d| d.flex_col()).child(h_flex().flex_1().min_w_0().gap(px(8.))
                .child(Checkbox::new(SharedString::from(format!("check-{id}"))).checked(task.status == "done").accessibility_label(format!("Mark {} {}",task.title,next)).tooltip("Toggle task completion").disabled(self.busy).on_click(cx.listener(move |this, _: &bool, _, cx| this.change("tasks.update", json!({"id": move_id, "status": next}), cx))))
                .child(Button::new(SharedString::from(format!("edit-{id}"))).label(task.title.clone()).ghost().flex_1().min_w_0().disabled(self.busy).tooltip("Edit task").on_click(cx.listener(move |this, _, window, cx| {
                    if this.busy || this.draft_dirty(cx) { this.error = Some("Save or discard your draft before opening another task.".into()); cx.notify(); return; }
                    this.editor_open = true; this.editing = Some(edit.id.clone()); this.editing_baseline = Some(edit.clone()); this.status = edit.status.clone(); this.project = edit.project.clone();
                    this.title.update(cx, |field, cx| field.set_value(edit.title.clone(), window, cx));
                    this.notes.update(cx, |field, cx| field.set_value(edit.notes.clone(), window, cx)); cx.notify();
                }))))
                .child(status_menu)
                .child(Button::new(SharedString::from(format!("delete-{id}"))).label("Delete").ghost().tooltip("Delete task; Undo stays available").disabled(self.busy).on_click(cx.listener(move |this, _, _, cx| { this.change("tasks.remove", json!({"id": delete_id}), cx); }))))
            .into_any_element()
    }
}
impl Render for TasksPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view.as_deref().or(self.prefs["defaultView"].as_str()).unwrap_or("board").to_string();
        let mut header = h_flex().flex_wrap().gap(px(10.)).child(div().text_size(px(24.)).child("Tasks".test_text("Tasks")));
        for name in ["list", "board"] {
            header = header.child(Button::new(name).selected(view == name).label(if name == "list" { "List" } else { "Board" }).tooltip("Switch task view").on_click(cx.listener(move |this, _, _, cx| { this.view = Some(name.into()); cx.notify(); })));
        }
        let default_view = view.clone();
        let is_default = self.prefs["defaultView"].as_str().unwrap_or("board") == view;
        header = header.child(Button::new("tasks-default").label(if is_default { "Default view" } else { "Set as default" }).tooltip("Open Tasks in this view next time").disabled(self.busy || is_default).on_click(cx.listener(move |this, _, _, cx| this.change("tasks.setPreferences", json!({"defaultView": default_view}), cx))));
        let web = self.app.read_with(cx, |app, _| app.website_url.clone()).ok().flatten();
        if let Some(url) = web { header = header.child(Button::new("tasks-web").label("Open on web").tooltip("Open the synced Tasks page in your browser").on_click(move |_, _, cx| { crate::open_gate::open_web(&format!("{}/tasks", url.trim_end_matches('/')), cx); })); }
        header = header.child(Button::new("tasks-customize").label("Customize").tooltip("Change the Tasks background").on_click(cx.listener(|this, _, window, cx| {
            this.customize = !this.customize;
            if this.customize { let color = this.prefs["backgroundColor"].as_str().unwrap_or("").to_string(); this.color.update(cx, |f,cx| f.set_value(color,window,cx)); }
            cx.notify();
        })));
        let app = self.app.clone();
        header = header.child(Button::new("tasks-close").label("Close").tooltip("Close Tasks (Esc)").on_click(move |_,_,cx| { app.update(cx, |app,cx| { app.close_tasks(cx); }).ok(); }));
        let mut page = v_flex().id("tasks-page").test_support().key_context("TasksPage").track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &CloseTasks, _, cx| { let app = this.app.clone(); cx.defer(move |cx| { app.update(cx, |app,cx| { app.close_tasks(cx); }).ok(); }); })).size_full().gap(px(12.)).p(px(16.)).text_color(style::label_text(cx)).bg(cx.theme().background).child(header);
        if let Some(error) = &self.error { page = page.child(div().child(error.clone().test_text(error.clone()))); }
        if self.busy { page = page.child("Saving…"); }
        let mut filters = vec![("All projects".into(), json!("all"))]; filters.extend(self.project_choices());
        let filter_name = if self.filter == json!("all") { "All projects".into() } else { self.project_name(&self.filter) };
        page = page.child(h_flex().flex_wrap().gap(px(8.))
            .child(self.picker("tasks-filter", filter_name, filters, "filter", cx))
            .child(self.picker("tasks-sort", format!("Sort: {}", self.prefs["sort"].as_str().unwrap_or("manual")), ["manual", "newest", "oldest", "title"].into_iter().map(|s| (s.into(), json!(s))).collect(), "sort", cx))
            .child(self.picker("tasks-group", format!("Group: {}", self.prefs["grouping"].as_str().unwrap_or("status")), ["status", "project"].into_iter().map(|s| (s.into(), json!(s))).collect(), "grouping", cx)));
        if self.customize {
            page = page.child(v_flex().gap(px(8.))
                .child("Background color (hex)").child(Input::new(&self.color).disabled(self.busy))
                .child(Button::new("tasks-color").label("Apply color").tooltip("Use a six-digit hex background color").disabled(self.busy).on_click(cx.listener(|this, _, _, cx| this.change("tasks.setPreferences", json!({"backgroundColor":this.color.read(cx).value().to_string()}), cx))))
                .child(div().id("tasks-photo-drop").p(px(12.)).border_1().border_color(cx.theme().border).child("Drop a photo here (up to 10 MB)").on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| { if let Some(path) = paths.paths().first() { this.change("tasks.photoFile", json!({"path":path.to_string_lossy()}), cx); } })))
                .child(self.picker("tasks-fade", "Photo fade".into(), [("Clear",0.0),("Light",0.4),("Soft",0.7),("Faint",0.9)].into_iter().map(|(s,v)| (s.into(),json!(v))).collect(), "fade", cx))
                .child(self.picker("tasks-position", "Photo position".into(), ["top","center","bottom"].into_iter().map(|s|(s.into(),json!(s))).collect(), "photoPosition", cx))
                .child(Button::new("tasks-reset-background").label("Use theme background").tooltip("Remove the custom color and photo").on_click(cx.listener(|this, _, _, cx| this.change("tasks.setPreferences", json!({"backgroundColor":"","backgroundPhoto":null}), cx)))));
        }
        let mut form = v_flex().gap(px(8.)).child("Title").child(Input::new(&self.title).disabled(self.busy)).child("Notes").child(Textarea::new(&self.notes).disabled(self.busy));
        let mut choices = h_flex().flex_wrap().gap(px(8.));
        for (status, label) in [("backlog", "Backlog"), ("todo", "Todo"), ("done", "Done")] {
            choices = choices.child(Button::new(SharedString::from(format!("new-{status}"))).label(label).selected(self.status == status).disabled(self.busy).tooltip("Status for this task").on_click(cx.listener(move |this, _, _, cx| { this.status = status.into(); cx.notify(); })));
        }
        let project_name = self.project_name(&self.project.clone().unwrap_or(Value::Null));
        choices = choices.child(self.picker("task-project", project_name, self.project_choices(), "project", cx));
        let app = self.app.clone();
        choices = choices.child(Button::new("task-new-project").label("Create project").tooltip("Create a Threadbear project, then select it here").on_click(move |_, window, cx| { app.update(cx, |app, cx| app.new_project(None, window, cx)).ok(); }));
        choices = choices.child(Button::new("task-new").label(if self.draft_dirty(cx) { "Discard draft" } else { "New task" }).tooltip("Clear the editor for a new task").disabled(self.busy).on_click(cx.listener(|this, _, window, cx| {
            this.editing = None; this.editing_baseline = None; this.draft_id = threadbear_core::notes::new_id();
            this.title.update(cx, |f,cx| f.set_value("",window,cx)); this.notes.update(cx,|f,cx| f.set_value("",window,cx)); cx.notify();
        })));
        form = form.child(choices).child(Button::new("save-task").label(if self.editing.is_some() { "Save task" } else { "Add task" }).tooltip("Save the task to your account").disabled(self.busy).on_click(cx.listener(|this, _, window, cx| this.save(window, cx))));
        page = page.child(Button::new("tasks-editor-toggle").label(if self.editor_open { "Hide editor" } else { "New task" }).tooltip("Show or hide the task editor; your draft is kept").on_click(cx.listener(|this,_,_,cx| { this.editor_open = !this.editor_open; cx.notify(); })));
        if self.editor_open { page = page.child(form); }
        if let Some(id) = self.undo.clone() { page = page.child(Button::new("undo-task").label("Undo deletion").tooltip("Restore the last deleted task").disabled(self.busy).on_click(cx.listener(move |this, _, _, cx| this.change("tasks.restore", json!({"id": id}), cx)))); }
        let order = self.prefs["columnOrder"].as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_else(|| vec!["backlog".into(), "todo".into(), "done".into()]);
        let groups: Vec<Value> = if self.prefs["grouping"] == "project" {
            let mut groups = vec![];
            for task in &self.rows { let p = task.project.clone().unwrap_or(Value::Null); if !groups.contains(&p) { groups.push(p); } }
            if groups.is_empty() { groups.push(Value::Null); } groups
        } else { vec![json!("all")] };
        let mut all_groups = v_flex().flex_1().min_h_0().gap(px(16.));
        for group in groups {
        let mut columns = div().id(SharedString::from(format!("tasks-columns-{group}"))).overflow_x_scroll().flex().gap(px(16.)).min_h(px(200.));
        if view == "list" { columns = columns.flex_col(); }
        if group != json!("all") { all_groups = all_groups.child(self.project_name(&group)); }
        for status in order.clone() {
            let drop_status = status.clone(); let column_target = status.clone(); let drop_project = group.clone();
            let mut rows: Vec<_> = self.rows.iter().filter(|r| r.status == status && (self.filter == json!("all") || r.project.clone().unwrap_or(Value::Null) == self.filter) && (group == json!("all") || r.project.clone().unwrap_or(Value::Null) == group)).collect();
            rows.sort_by(|a,b| match self.prefs["sort"].as_str().unwrap_or("manual") { "title" => a.title.cmp(&b.title), "newest" => b.created_at.total_cmp(&a.created_at), "oldest" => a.created_at.total_cmp(&b.created_at), _ => a.position.total_cmp(&b.position) });
            let add_status = status.clone();
            let add_project = if group == json!("all") { self.filter.clone() } else { group.clone() };
            let end_position = rows.iter().map(|r| r.position).max_by(f64::total_cmp).map_or(crate::now_ms() as f64, |p| p + 1000.);
            let mut column = v_flex().id(SharedString::from(format!("column-{group}-{status}"))).flex_1().min_w(px(250.)).p(px(12.)).rounded(px(8.)).bg(cx.theme().muted)
                .on_drop(cx.listener(move |this, drag: &TaskDrag, _, cx| {
                    let mut args = json!({"id": drag.id, "status": drop_status, "position": end_position});
                    if drop_project != json!("all") { args["project"] = drop_project.clone(); }
                    this.change("tasks.update", args, cx);
                }))
                .on_drop(cx.listener(move |this, drag: &ColumnDrag, _, cx| {
                    if drag.0 == column_target { return; }
                    let mut order = this.prefs["columnOrder"].as_array().cloned().unwrap_or_else(|| vec![json!("backlog"),json!("todo"),json!("done")]);
                    order.retain(|v| v.as_str() != Some(&drag.0));
                    let index = order.iter().position(|v| v.as_str() == Some(&column_target)).unwrap_or(order.len());
                    order.insert(index,json!(drag.0)); this.change("tasks.setPreferences",json!({"columnOrder":order}),cx);
                }))
                .child(div().id(SharedString::from(format!("column-drag-{group}-{status}"))).mb(px(12.)).cursor_pointer()
                    .on_drag(ColumnDrag(status.clone()), |drag,_,_,cx| cx.new(|_| TaskGhost(drag.0.clone())))
                    .child(format!("{}  {}", match status.as_str() { "backlog" => "Backlog", "done" => "Done", _ => "Todo" }, rows.len())));
            column = column.child(Button::new(SharedString::from(format!("add-{group}-{status}"))).label("Add task").ghost().disabled(self.busy).tooltip("Add a task in this column")
                .on_click(cx.listener(move |this,_,window,cx| {
                    if this.draft_dirty(cx) { this.error = Some("Save or discard your draft before starting another task.".into()); cx.notify(); return; }
                    this.editing = None; this.editing_baseline = None; this.draft_id = threadbear_core::notes::new_id(); this.status = add_status.clone(); this.editor_open = true;
                    this.project = add_project.is_object().then(|| add_project.clone()); this.error = None;
                    this.title.update(cx, |f,cx| { f.set_value("",window,cx); f.focus(window,cx); });
                    this.notes.update(cx, |f,cx| f.set_value("",window,cx)); cx.notify();
                })));
            for task in rows { column = column.child(self.card(task, cx)); }
            columns = columns.child(column);
        }
        all_groups = all_groups.child(columns);
        }
        let bg = self.prefs["backgroundColor"].as_str().and_then(|c| u32::from_str_radix(c.trim_start_matches('#'),16).ok()).map(|c| Hsla::from(rgb(c))).unwrap_or(cx.theme().background);
        let mut shell = div().relative().size_full().overflow_hidden().bg(bg);
        if let Some(url) = &self.photo {
            let resource = Resource::Uri(url.clone().into());
            if let Some(Ok(image)) = window.use_asset::<ImgResourceLoader>(&resource, cx) {
                let size = image.size(0);
                let y = match self.prefs["photoPosition"].as_str() { Some("top") => 0., Some("bottom") => 1., _ => 0.5 };
                let (x,y,w,h) = threadbear_core::looks::BackdropFocus::new(0.5,y).frame((u32::from(size.width) as f32,u32::from(size.height) as f32),self.background_size);
                shell = shell.child(img(url.clone()).absolute().left(px(x)).top(px(y)).w(px(w)).h(px(h)).object_fit(ObjectFit::Fill).opacity((1. - self.prefs["fade"].as_f64().unwrap_or(0.85)) as f32));
            }
            let page = cx.entity().downgrade();
            shell = shell.child(canvas(move |bounds,_,cx| {
                let one = style::zoomed(1.);
                let size = (f32::from(bounds.size.width)/one,f32::from(bounds.size.height)/one);
                let page = page.clone();
                cx.defer(move |cx| { page.update(cx, |p,cx| {
                    if (p.background_size.0-size.0).abs()>0.5 || (p.background_size.1-size.1).abs()>0.5 { p.background_size=size; cx.notify(); }
                }).ok(); });
            }, |_,_,_,_| {}).absolute().inset_0().size_full());
        }
        shell.child(page.bg(gpui_kit::transparent_black()).child(div().id("tasks-scroll").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.scroll).child(all_groups)))
    }
}

/// Scripted commands enter through the same editor and save path as clicks.
pub(crate) fn automation(app: &Entity<Threadbear>, cmd: &Value, window: &mut Window, cx: &mut App) -> Result<Value, String> {
    if app.read(cx).tasks_page.is_none() { app.update(cx, |app,cx| app.toggle_tasks(window,cx)); }
    let page = app.read(cx).tasks_page.clone().ok_or("Tasks experiment is off")?;
    page.update(cx, |p,cx| {
        if let Some(title) = cmd["title"].as_str() { p.editor_open = true; p.title.update(cx, |f,cx| f.set_value(title.to_owned(),window,cx)); }
        if let Some(notes) = cmd["notes"].as_str() { p.notes.update(cx, |f,cx| f.set_value(notes.to_owned(),window,cx)); }
        if cmd["save"].as_bool() == Some(true) { p.save(window,cx); }
        cx.notify();
    });
    Ok(json!("ok"))
}
