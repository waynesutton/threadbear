// Bears & Tables, a panel on the right rail while the bears experiment is on
// (prds/bears-gpui.md): this Mac's bears from `bears.list` (Pinned, Bears,
// Archived, the head bear marked) and, with tables on too, its tables, the
// projects with a `table`. It takes the place of the home patch's list
// panel and looks the same, with row menus and picking on top:
//
// - A click opens the bear's or table's newest thread; a bear with none
//   shows its CLAUDE.md here, a table with none its project page.
// - Right-click: Rename… (`bears.update { bear, name }`, or for a table,
//   a project, `projects.update { projectId, title }`) and Delete… (a
//   sheet, then `bears.delete`, or `projects.delete` for a table: its chat
//   threads and folder stay).
// - ⌘-click picks rows, Shift-click a run of them, Esc lets go. With some
//   picked, a bar over the list and the menu offer Delete N, one sheet for
//   them all.
//
// Every write shows at once and goes back, with the error, if the runner
// refuses (AGENTS.md, "Assume every action is slow to land").
use crate::app::Threadbear;
use crate::highlights::{icon, palette, tipped, Heading, PaneRow, Palette, SidePane};
use crate::sidebar::{MenuAction, MenuRow};
use crate::style::{self, px};
use crate::tips::Tip as _;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{h_flex, v_flex, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::json;
#[cfg(feature = "automation")]
use serde_json::Value;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use threadbear_core::bears_panel::{self as rules, Click, PanelBear, Selection};
use threadbear_core::inbox::ThreadSummary;
use threadbear_core::projects::Update;

pub(crate) const BEARS: &str = "threadbear.bears";

/// How often the list is asked for again while it's drawn, as the patch's
/// list panel was.
const REFRESH: Duration = Duration::from_secs(15);
const RETRY: Duration = Duration::from_secs(3);

#[derive(Default)]
pub(crate) struct BearsPanelState {
    /// `bears.list`'s last answer, archived ones too; none until it comes.
    pub(crate) bears: Option<Vec<PanelBear>>,
    fetched_at: Option<Instant>,
    fetching: bool,
    pub(crate) selection: Selection,
    /// Rows on their way out: hidden until the runner says, back if it refuses.
    deleting: HashSet<String>,
    /// The list's own focus, for Esc.
    focus: Option<FocusHandle>,
}

/// What a line of the panel's menu does (sidebar.rs's `MenuAction::Bears`).
#[derive(Clone, Debug)]
pub(crate) enum BearsAction {
    Rename(String),
    Delete(Vec<String>),
    ClearSelection,
}

/// What a click on a row opens.
#[derive(Clone, Debug)]
pub(crate) enum Opens {
    Thread(String),
    File(String),
    Project(String),
}

/// One row, as drawn and as the dump gives it.
#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub(crate) key: String,
    pub(crate) section: &'static str,
    pub(crate) name: String,
    pub(crate) title: String,
    pub(crate) detail: Option<String>,
    pub(crate) icon: String,
    pub(crate) working: bool,
    pub(crate) trailing: Option<String>,
    opens: Option<Opens>,
}

fn expand_home(path: &str) -> String {
    match path.strip_prefix("~") {
        Some(rest) => format!("{}{rest}", std::env::var("HOME").unwrap_or_default()),
        None => path.to_owned(),
    }
}

fn in_folder(cwd: &str, folder: &str) -> bool {
    !folder.is_empty() && (cwd == folder || cwd.strip_prefix(folder).is_some_and(|rest| rest.starts_with('/')))
}

fn is_working(t: &ThreadSummary) -> bool {
    matches!(t.status.as_str(), "running" | "starting")
}

/// "Away until Mon 9:00 AM", in this Mac's time.
fn away_text(iso: &str) -> String {
    use threadbear_core::chrono::{DateTime, Local};
    match DateTime::parse_from_rfc3339(iso) {
        Ok(at) => format!("Away until {}", at.with_timezone(&Local).format("%a %-I:%M %p")),
        Err(_) => "Away".to_owned(),
    }
}

impl Threadbear {
    pub(crate) fn bears_on(&self) -> bool {
        self.remote.experiments.iter().any(|e| e == "bears")
    }

    pub(crate) fn tables_on(&self) -> bool {
        self.bears_on() && self.remote.experiments.iter().any(|e| e == "tables")
    }

    /// The rail's button and the panel: while bears are on, for a thread on
    /// this Mac (the bears are this Mac's runner's).
    pub(crate) fn shows_bears_rail(&self, thread: &ThreadSummary) -> bool {
        self.bears_on() && !thread.is_remote()
    }

    pub(crate) fn bears_panel_stale(&self) -> bool {
        !self.bears_panel.fetching && self.bears_panel.fetched_at.is_none_or(|at| at.elapsed() >= REFRESH)
    }

    /// Asks the runner for its bears, archived ones too, unless it did lately.
    pub(crate) fn refresh_bears_panel(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.bears_panel.focus.is_none() {
            self.bears_panel.focus = Some(cx.focus_handle());
        }
        if self.bears_panel.fetching || (!force && !self.bears_panel_stale()) {
            return;
        }
        self.bears_panel.fetching = true;
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = crate::call(client, "bears.list", json!({ "archived": true, "faces": false })).await;
            this.update(cx, |this, cx| {
                let state = &mut this.bears_panel;
                state.fetching = false;
                // One that fails is asked again in a few seconds (before
                // the app's connected, say), not left at Loading.
                let wait = if res.is_ok() { REFRESH } else { RETRY };
                state.fetched_at = Some(Instant::now() - (REFRESH - wait));
                if let Ok(value) = res {
                    state.bears = Some(rules::decode(&value).unwrap_or_default());
                }
                let order: Vec<String> = this.bears_panel_rows().into_iter().map(|r| r.key).collect();
                this.bears_panel.selection.retain(&order);
                cx.notify();
                // Drawn again then, which asks again while the panel's on show.
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(wait).await;
                    this.update(cx, |_, cx| cx.notify()).ok();
                })
                .detach();
            })
            .ok();
        })
        .detach();
    }

    /// The rows down the panel, in order.
    pub(crate) fn bears_panel_rows(&self) -> Vec<Row> {
        let state = &self.bears_panel;
        let live: Vec<&ThreadSummary> = self.threads.values().map(|t| &**t).filter(|t| !t.is_archived()).collect();
        let newest = |pick: &dyn Fn(&ThreadSummary) -> bool| live.iter().copied().filter(|t| pick(t)).max_by_key(|t| t.activity_ms()).cloned();
        let bears = state.bears.clone().unwrap_or_default();
        let mut rows = vec![];
        for bear in &bears {
            let key = rules::bear_key(&bear.id);
            if state.deleting.contains(&key) {
                continue;
            }
            let id = bear.id.clone();
            let folder = bear.folder.clone();
            let thread = newest(&|t: &ThreadSummary| t.bear.as_ref().is_some_and(|b| b.id == id) || in_folder(&t.cwd, &folder));
            let working = thread.as_ref().is_some_and(is_working);
            let detail = match (&bear.away_until, &thread) {
                (Some(until), _) => Some(away_text(until)),
                (None, Some(t)) if working => Some(t.title.clone()),
                _ => bear.does.clone(),
            };
            let opens = match &thread {
                Some(t) => Some(Opens::Thread(t.id.clone())),
                None if !bear.folder.is_empty() => Some(Opens::File(format!("{}/CLAUDE.md", bear.folder))),
                None => None,
            };
            rows.push(Row {
                key,
                section: bear.section(),
                name: bear.name.clone(),
                title: bear.title(),
                detail,
                icon: if bear.head { "crown".to_owned() } else { bear.icon.clone().unwrap_or_else(|| "pawprint".to_owned()) },
                working,
                trailing: (bear.routines > 0).then(|| format!("{} routine{}", bear.routines, if bear.routines == 1 { "" } else { "s" })),
                opens,
            });
        }
        if self.tables_on() {
            for project in self.projects.list.iter().filter(|p| p.table.is_some()) {
                let key = rules::table_key(&project.id);
                if state.deleting.contains(&key) {
                    continue;
                }
                let folder = project.folders.first().map(|f| expand_home(f)).unwrap_or_default();
                let thread = newest(&|t: &ThreadSummary| in_folder(&t.cwd, &folder));
                let seated: Vec<String> = project.table.iter().flat_map(|t| t.bears.iter()).map(|b| bears.iter().find(|x| &x.id == b).map_or_else(|| b.clone(), |x| x.name.clone())).collect();
                rows.push(Row {
                    key,
                    section: if project.archived { rules::ARCHIVED } else { rules::TABLES },
                    name: project.title.clone(),
                    title: project.title.clone(),
                    detail: Some(if seated.is_empty() { "No bears yet".to_owned() } else { format!("With {}", seated.join(", ")) }),
                    icon: "person.3".to_owned(),
                    working: thread.as_ref().is_some_and(is_working),
                    trailing: None,
                    opens: Some(match thread {
                        Some(t) => Opens::Thread(t.id.clone()),
                        None => Opens::Project(project.id.clone()),
                    }),
                });
            }
        }
        rows.sort_by_key(|r| rules::section_rank(r.section));
        rows
    }

    /// A click on a row: plain opens it, ⌘ and Shift pick.
    pub(crate) fn bears_panel_click(&mut self, key: &str, click: Click, window: &mut Window, cx: &mut Context<Self>) -> Option<Opens> {
        let rows = self.bears_panel_rows();
        let order: Vec<String> = rows.iter().map(|r| r.key.clone()).collect();
        let opens = self.bears_panel.selection.click(key, click, &order);
        if let Some(focus) = self.bears_panel.focus.clone() {
            window.focus(&focus, cx);
            crate::web_panels::keyboard_to_app(window);
        }
        cx.notify();
        if !opens {
            return None;
        }
        let opens = rows.into_iter().find(|r| r.key == key)?.opens?;
        match &opens {
            Opens::Thread(id) => self.open(id.clone(), window, cx),
            Opens::Project(id) => self.open_project_page(id, cx),
            // Shown in the pane, by the pane (`bears_row_click`).
            Opens::File(_) => {}
        }
        Some(opens)
    }

    pub(crate) fn clear_bears_selection(&mut self, cx: &mut Context<Self>) -> bool {
        if self.bears_panel.selection.is_empty() {
            return false;
        }
        self.bears_panel.selection.clear();
        cx.notify();
        true
    }

    /// The row's menu: Rename… and Delete…, or with rows picked and this one
    /// among them, Delete N items… for them all. A right-click on a row
    /// that isn't picked lets the others go, so the menu is plainly its.
    pub(crate) fn open_bears_menu(&mut self, key: &str, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.bears_menu_rows(key);
        if rows.is_empty() {
            return;
        }
        self.open_menu(None, rows, position, false, window, cx);
    }

    pub(crate) fn bears_menu_rows(&mut self, key: &str) -> Vec<MenuRow> {
        let picked = &self.bears_panel.selection;
        if picked.len() > 1 && picked.contains(key) {
            let keys = picked.keys.clone();
            return vec![
                MenuRow::item(format!("Delete {} items\u{2026}", keys.len()), "trash", MenuAction::Bears(BearsAction::Delete(keys))),
                MenuRow::separator(),
                MenuRow::item("Clear selection", "xmark", MenuAction::Bears(BearsAction::ClearSelection)),
            ];
        }
        if !picked.contains(key) {
            self.bears_panel.selection.clear();
        }
        vec![
            MenuRow::item("Rename\u{2026}", "pencil", MenuAction::Bears(BearsAction::Rename(key.to_owned()))),
            MenuRow::item("Delete\u{2026}", "trash", MenuAction::Bears(BearsAction::Delete(vec![key.to_owned()]))),
        ]
    }

    pub(crate) fn run_bears_action(&mut self, action: BearsAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            BearsAction::Rename(key) => self.open_bears_rename(&key, window, cx),
            BearsAction::Delete(keys) => self.open_bears_delete(keys, window, cx),
            BearsAction::ClearSelection => {
                self.clear_bears_selection(cx);
            }
        }
    }

    fn bears_row_name(&self, key: &str) -> Option<String> {
        if let Some(id) = key.strip_prefix("bear:") {
            return self.bears_panel.bears.as_ref()?.iter().find(|b| b.id == id).map(|b| b.name.clone());
        }
        let id = key.strip_prefix("table:")?;
        self.project(id).map(|p| p.title.clone())
    }

    /// Rename…: a box with the name in it.
    fn open_bears_rename(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = self.bears_row_name(key) else { return };
        let is_bear = key.starts_with("bear:");
        let entity = cx.entity().downgrade();
        let field = cx.new(|cx| InputState::new(window, cx).placeholder(if is_bear { "Bear's name" } else { "Table's name" }).default_value(name.clone()));
        let field_focus = field.clone();
        let key = key.to_owned();
        let title = format!("Rename \u{201C}{name}\u{201D}");
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let (entity, field_read, key, before) = (entity.clone(), field.clone(), key.clone(), name.clone());
            dialog
                .title(title.clone())
                .width(px(420.))
                .child(div().id("bears-rename-field").tip("The new name").child(Input::new(&field)))
                .button_props(DialogButtonProps::default().ok_text("Rename").show_cancel(true))
                .on_ok(move |_, _, cx| {
                    let said = field_read.read(cx).value().trim().to_string();
                    if !said.is_empty() && said != before {
                        let key = key.clone();
                        entity.update(cx, |this, cx| this.rename_bears_row(&key, said, cx)).ok();
                    }
                    true
                })
        });
        let handle = field_focus.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
        crate::web_panels::keyboard_to_app(window);
    }

    /// The new name shows at once; the old one comes back if the runner refuses.
    pub(crate) fn rename_bears_row(&mut self, key: &str, name: String, cx: &mut Context<Self>) {
        if let Some(id) = key.strip_prefix("table:") {
            self.update_project(id, Update { title: Some(name), ..Default::default() }, cx);
            return;
        }
        let Some(id) = key.strip_prefix("bear:").map(str::to_owned) else { return };
        let busy = format!("bearRename/{id}");
        let Some(before) = self.bears_row_name(key) else { return };
        if !self.begin_note(&busy, &format!("Renaming {before}\u{2026}"), cx) {
            return;
        }
        self.set_panel_bear_name(&id, &name);
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = crate::call(client, "bears.update", json!({ "bear": id, "name": name })).await;
            this.update(cx, |this, cx| {
                this.end_note(&busy, cx);
                match res {
                    Ok(v) => {
                        if let Some(named) = v["bear"]["name"].as_str() {
                            this.set_panel_bear_name(&id, named);
                        }
                        this.bears_changed();
                        this.refresh_bears_panel(true, cx);
                    }
                    Err(e) => {
                        this.set_panel_bear_name(&id, &before);
                        this.show_error(e.message, cx);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn set_panel_bear_name(&mut self, id: &str, name: &str) {
        if let Some(bear) = self.bears_panel.bears.iter_mut().flatten().find(|b| b.id == id) {
            bear.name = name.to_owned();
        }
    }

    /// Delete…: one sheet naming everything that goes.
    pub(crate) fn open_bears_delete(&mut self, keys: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        let mut bears = vec![];
        let mut tables = vec![];
        for key in &keys {
            let Some(name) = self.bears_row_name(key) else { continue };
            if key.starts_with("bear:") { bears.push(name) } else { tables.push(name) }
        }
        if bears.is_empty() && tables.is_empty() {
            return;
        }
        let (title, text) = rules::delete_words(&bears, &tables);
        let entity = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (entity, keys) = (entity.clone(), keys.clone());
            alert
                .title(title.clone())
                .description(text.clone())
                .button_props(DialogButtonProps::default().ok_text("Delete").show_cancel(true))
                .on_ok(move |_, _, cx| {
                    let keys = keys.clone();
                    entity.update(cx, |this, cx| this.delete_bears_rows(keys, cx)).ok();
                    true
                })
        });
    }

    /// Gone from the list at once; what the runner refuses comes back, with why.
    pub(crate) fn delete_bears_rows(&mut self, keys: Vec<String>, cx: &mut Context<Self>) {
        self.bears_panel.selection.clear();
        let bears: Vec<String> = keys.iter().filter_map(|k| k.strip_prefix("bear:").map(str::to_owned)).collect();
        for id in keys.iter().filter_map(|k| k.strip_prefix("table:")) {
            self.confirmed_delete_project(id, cx);
        }
        if bears.is_empty() {
            cx.notify();
            return;
        }
        let busy = format!("bearsDelete/{}", bears.join(","));
        let note = if bears.len() == 1 { format!("Deleting {}\u{2026}", self.bears_row_name(&rules::bear_key(&bears[0])).unwrap_or_default()) } else { format!("Deleting {} bears\u{2026}", bears.len()) };
        if !self.begin_note(&busy, &note, cx) {
            return;
        }
        for id in &bears {
            self.bears_panel.deleting.insert(rules::bear_key(id));
        }
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            // One at a time: each rewrites the runner's bears file.
            let mut failed = vec![];
            for id in &bears {
                if let Err(e) = crate::call(client.clone(), "bears.delete", json!({ "bear": id })).await {
                    failed.push((id.clone(), e.message));
                }
            }
            this.update(cx, |this, cx| {
                this.end_note(&busy, cx);
                for id in &bears {
                    this.bears_panel.deleting.remove(&rules::bear_key(id));
                    if !failed.iter().any(|(f, _)| f == id) {
                        if let Some(list) = this.bears_panel.bears.as_mut() {
                            list.retain(|b| &b.id != id);
                        }
                    }
                }
                if let Some((_, message)) = failed.first() {
                    let message = if failed.len() > 1 { format!("{} bears weren't deleted: {message}", failed.len()) } else { message.clone() };
                    this.show_error(message, cx);
                }
                this.bears_changed();
                this.refresh_bears_panel(true, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The `bearsPanel` part of the dump.
    #[cfg(feature = "automation")]
    pub(crate) fn bears_panel_dump(&self) -> Value {
        let rows: Vec<Value> = self
            .bears_panel_rows()
            .into_iter()
            .map(|r| json!({ "key": r.key, "section": r.section, "title": r.title, "detail": r.detail, "picked": self.bears_panel.selection.contains(&r.key) }))
            .collect();
        json!({ "on": self.bears_on(), "tables": self.tables_on(), "loaded": self.bears_panel.bears.is_some(), "rows": rows, "picked": self.bears_panel.selection.keys })
    }
}

/// `{"cmd":"bears", ...}`: `"click":"<key>"` with `"how":"toggle"|"range"`
/// (plain otherwise), `"menu":"<key>"` (the menu's lines, opened at `x`, `y`
/// if given), `"rename":"<key>","name":"..."` and `"delete":[keys]` (the
/// sheet, as the menu opens it), `"clear":true`; with nothing, the dump.
#[cfg(feature = "automation")]
pub(crate) fn automation(app: &Entity<Threadbear>, cmd: &Value, window: &mut Window, cx: &mut App) -> Result<Value, String> {
    if let Some(key) = cmd["click"].as_str() {
        let how = match cmd["how"].as_str() {
            Some("toggle") => Click::Toggle,
            Some("range") => Click::Range,
            _ => Click::Plain,
        };
        let key = key.to_owned();
        let opened = app.update(cx, |t, cx| t.bears_panel_click(&key, how, window, cx));
        if let Some(Opens::File(path)) = opened {
            let side = app.read(cx).side.clone();
            if let Some(thread) = side.read(cx).thread(cx) {
                side.update(cx, |s, cx| s.preview_path(&path, Some("CLAUDE.md".to_owned()), &thread, cx));
            }
        }
        return Ok(app.read(cx).bears_panel_dump());
    }
    if let Some(key) = cmd["menu"].as_str() {
        let key = key.to_owned();
        let at = point(px(cmd["x"].as_f64().unwrap_or(600.) as f32), px(cmd["y"].as_f64().unwrap_or(200.) as f32));
        let titles: Vec<String> = app.update(cx, |t, cx| {
            let rows = t.bears_menu_rows(&key);
            let titles = rows.iter().map(|r| r.title.clone()).filter(|t| !t.is_empty()).collect();
            t.open_bears_menu(&key, at, window, cx);
            titles
        });
        return Ok(json!(titles));
    }
    if let (Some(key), Some(name)) = (cmd["rename"].as_str(), cmd["name"].as_str()) {
        let (key, name) = (key.to_owned(), name.to_owned());
        app.update(cx, |t, cx| t.rename_bears_row(&key, name, cx));
        return Ok(json!("ok"));
    }
    if let Some(keys) = cmd["delete"].as_array() {
        let keys: Vec<String> = keys.iter().filter_map(|k| k.as_str().map(str::to_owned)).collect();
        app.update(cx, |t, cx| t.open_bears_delete(keys, window, cx));
        return Ok(json!("ok sheet open"));
    }
    if cmd["clear"].as_bool() == Some(true) {
        app.update(cx, |t, cx| t.clear_bears_selection(cx));
    }
    if cmd["refresh"].as_bool() == Some(true) {
        app.update(cx, |t, cx| t.refresh_bears_panel(true, cx));
    }
    Ok(app.read(cx).bears_panel_dump())
}

impl SidePane {
    /// The panel itself, drawn as the patch's list panel was.
    pub(crate) fn render_bears_panel(&mut self, thread: &ThreadSummary, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let Some(app) = self.app.upgrade() else { return div().into_any_element() };
        let (rows, loaded, picked, focus, stale) = {
            let a = app.read(cx);
            let state = &a.bears_panel;
            (a.bears_panel_rows(), state.bears.is_some(), state.selection.clone(), state.focus.clone(), a.bears_panel_stale() || state.focus.is_none())
        };
        if stale {
            let app = app.downgrade();
            cx.defer(move |cx| {
                app.update(cx, |a, cx| a.refresh_bears_panel(false, cx)).ok();
            });
        }
        if rows.is_empty() {
            let note = if loaded { "No bears yet. Ask any thread to make one." } else { "Loading\u{2026}" };
            return crate::files::pane_note(note, p).into_any_element();
        }
        let reserves = true;
        let mut list = v_flex().py(px(8.));
        let mut last: Option<&'static str> = None;
        for row in &rows {
            if last != Some(row.section) {
                last = Some(row.section);
                list = list.child(self.heading(Heading { key: format!("bears-{}", row.section), title: row.section.to_owned(), icon: None, count: None, folded: false, foldable: false, accessory: None }, p, cx));
            }
            list = list.child(self.bears_row(row, picked.contains(&row.key), reserves, thread, p, cx));
        }
        let bar = (!picked.is_empty()).then(|| self.bears_bar(picked.keys.clone(), p, cx));
        let app_weak = app.downgrade();
        let mut body = v_flex()
            .id("bears-panel")
            .size_full()
            .on_key_down(cx.listener(move |_, ev: &KeyDownEvent, _, cx| {
                if ev.keystroke.key == "escape" && app_weak.update(cx, |a, cx| a.clear_bears_selection(cx)).unwrap_or(false) {
                    cx.stop_propagation();
                }
            }))
            .children(bar)
            .child(div().id("bears-list").flex_1().min_h_0().overflow_y_scroll().child(list));
        if let Some(focus) = focus {
            body = body.track_focus(&focus);
        }
        body.into_any_element()
    }

    fn bears_row(&mut self, row: &Row, picked: bool, reserves: bool, thread: &ThreadSummary, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let tip = match &row.opens {
            Some(Opens::Thread(_)) => format!("Open {}'s newest thread. \u{2318}-click or Shift-click to pick more than one.", row.name),
            Some(Opens::File(_)) => format!("Show {}'s persona. \u{2318}-click or Shift-click to pick more than one.", row.name),
            Some(Opens::Project(_)) => format!("Open {}'s project page. \u{2318}-click or Shift-click to pick more than one.", row.name),
            None => format!("\u{2318}-click or Shift-click to pick {}", row.name),
        };
        let pane_row = PaneRow::new(SharedString::from(format!("bears-row-{}", row.key)), row.title.clone())
            .icon(row.icon.clone())
            .tinted(row.working.then_some(p.needs_you))
            .reserves_icon(reserves)
            .trailing(row.trailing.clone())
            .detail(row.detail.clone())
            .detail_lines(2)
            .tip(tip);
        let (key, menu_key, thread) = (row.key.clone(), row.key.clone(), thread.clone());
        let pane_row = pane_row
            .on_click(move |this, window, cx| {
                let m = window.modifiers();
                let how = if m.shift { Click::Range } else if m.platform || m.control { Click::Toggle } else { Click::Plain };
                let Some(app) = this.app.upgrade() else { return };
                let opened = app.update(cx, |a, cx| a.bears_panel_click(&key, how, window, cx));
                if let Some(Opens::File(path)) = opened {
                    this.preview_path(&path, Some("CLAUDE.md".to_owned()), &thread, cx);
                }
            })
            .on_menu(move |this, pos, window, cx| {
                if let Some(app) = this.app.upgrade() {
                    app.update(cx, |a, cx| a.open_bears_menu(&menu_key, pos, window, cx));
                }
            });
        pane_row.picked(picked.then(|| style::primary_opacity(cx, 0.12))).render(p, cx).into_any_element()
    }

    /// Over the list while rows are picked: how many, Delete N and Clear.
    fn bears_bar(&mut self, keys: Vec<String>, p: Palette, cx: &mut Context<Self>) -> AnyElement {
        let n = keys.len();
        let (fill, edge) = (style::button_fill(cx), style::button_edge(cx));
        let button = |id: &'static str, label: String, color: Hsla| div().id(id).flex_none().px(px(8.)).py(px(3.)).rounded(px(5.)).bg(fill).border_1().border_color(edge).cursor_pointer().child(div().text_size(px(style::size::DETAIL)).text_color(color).child(label));
        let delete = tipped(button("bears-delete-picked", format!("Delete {n}"), p.failed), format!("Delete the {n} picked, after asking")).on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
            if let Some(app) = this.app.upgrade() {
                let keys = keys.clone();
                app.update(cx, |a, cx| a.open_bears_delete(keys, window, cx));
            }
        }));
        let clear = tipped(button("bears-clear-picked", "Clear".to_owned(), p.fg), "Let go of the picked rows (Esc)").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            if let Some(app) = this.app.upgrade() {
                app.update(cx, |a, cx| a.clear_bears_selection(cx));
            }
        }));
        h_flex()
            .flex_none()
            .gap(px(8.))
            .items_center()
            .px(px(crate::highlights::LEADING))
            .py(px(8.))
            .border_b_1()
            .border_color(p.hairline)
            .child(icon("checkmark", 11., p.secondary))
            .child(div().flex_1().text_size(px(style::size::DETAIL)).text_color(p.secondary).child(format!("{n} picked")))
            .child(clear)
            .child(delete)
            .into_any_element()
    }
}
