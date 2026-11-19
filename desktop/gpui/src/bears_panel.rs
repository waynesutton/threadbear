// Bears & Tables, a panel on the right rail while the bears experiment is on
// (prds/bears-gpui.md): this Mac's bears from `bears.list` (Pinned, Bears,
// Archived, the head bear marked) and, with tables on too, its tables, the
// projects with a `table`. It takes the place of the home patch's list
// panel and looks the same, with row menus and picking on top:
//
// - A click opens the bear's or table's newest thread; a bear with none
//   shows its CLAUDE.md here, a table with none its project page.
// - Right-click: Open, Edit Settings… (the bear's bear.json, or a table's
//   project goals), Show in Finder, Copy Folder Path, Refresh and Rename…
//   (`bears.update { bear, name }`, or for a table,
//   a project, `projects.update { projectId, title }`) and Delete… (a
//   sheet, then `bears.delete`, or `projects.delete` for a table: its chat
//   threads and folder stay).
// - New (each section's controls), Change in a Thread…, Tables & Projects…
//   or Bears & Project…, and Set Photo… are bears_sheets.rs's. A row with a
//   photo draws it in place of its icon.
// - ⌘-click picks rows, Shift-click a run of them, Esc lets go. With some
//   picked, a bar over the list and the menu offer Delete N, one sheet for
//   them all.
//
// Every write shows at once and goes back, with the error, if the runner
// refuses (AGENTS.md, "Assume every action is slow to land").
use crate::app::Threadbear;
use crate::sidebar_lists::Kind;
use gpui_kit::base::TestTextExt as _;
use crate::highlights::{palette, PaneRow, Palette, SidePane};
use crate::sidebar::{MenuAction, MenuRow};
use crate::style::{self, px};
use crate::tips::Tip as _;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{v_flex, WindowExt as _};
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
    pub(crate) bears_view: crate::sidebar_lists::ListState,
    pub(crate) tables_view: crate::sidebar_lists::ListState,
    pub(crate) archiving: bool,
    /// `bears.list`'s last answer, archived ones too; none until it comes.
    pub(crate) bears: Option<Vec<PanelBear>>,
    fetched_at: Option<Instant>,
    fetching: bool,
    pub(crate) selection: Selection,
    pub(crate) table_selection: Selection,
    /// Rows on their way out: hidden until the runner says, back if it refuses.
    deleting: HashSet<String>,
    /// The list's own focus, for Esc.
    focus: Option<FocusHandle>,
}

impl BearsPanelState {
    pub(crate) fn selected(&self, kind: Kind) -> &Selection { if kind == Kind::Tables { &self.table_selection } else { &self.selection } }
    pub(crate) fn selected_mut(&mut self, kind: Kind) -> &mut Selection { if kind == Kind::Tables { &mut self.table_selection } else { &mut self.selection } }
    fn picked(&self, key: &str) -> bool { self.selected(if key.starts_with("table:") { Kind::Tables } else { Kind::Bears }).contains(key) }
}

/// What a line of the panel's menu does (sidebar.rs's `MenuAction::Bears`).
#[derive(Clone, Debug)]
pub(crate) enum BearsAction {
    Open(String),
    Edit(String),
    Reveal(String),
    CopyFolder(String),
    Refresh,
    Rename(String),
    Delete(Vec<String>),
    ClearSelection(Kind),
    /// A new thread that changes it (bears_sheets.rs).
    Change(String),
    /// A bear's tables and projects, or a table's bears and parent.
    Seats(String),
    SetPhoto(String),
    RemovePhoto(String),
    NewBear,
    NewTable,
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
    /// Its photo, drawn in place of the icon.
    pub(crate) picture: Option<std::path::PathBuf>,
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
                    let mut bears = rules::decode(&value).unwrap_or_default();
                    // A few small folders on this Mac, read as the list comes.
                    for bear in &mut bears {
                        bear.photo = rules::find_photo(&expand_home(&bear.folder));
                    }
                    state.bears = Some(bears);
                }
                let order: Vec<String> = this.bears_panel_rows().into_iter().map(|r| r.key).collect();
                this.bears_panel.selection.retain(&order);
                this.bears_panel.table_selection.retain(&order);
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
                picture: bear.photo.clone(),
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
                    picture: project.icon.as_ref().and_then(|i| i.image_path.clone()).map(std::path::PathBuf::from),
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
        let kind = if key.starts_with("table:") { Kind::Tables } else { Kind::Bears };
        if self.bears_panel.archiving { return None; }
        let rows = self.visible_bears(kind);
        let mut click = click;
        if self.sidebar_list(kind).selecting && click == Click::Plain { click = Click::Toggle; }
        if click != Click::Plain {
            if kind == Kind::Bears { self.bears_panel.bears_view.selecting = true; }
            else { self.bears_panel.tables_view.selecting = true; }
        }
        let order: Vec<String> = rows.iter().map(|r| r.key.clone()).collect();
        let opens = self.bears_panel.selected_mut(kind).click(key, click, &order);
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
        if self.bears_panel.selection.is_empty() && self.bears_panel.table_selection.is_empty() {
            return false;
        }
        self.bears_panel.selection.clear();
        self.bears_panel.table_selection.clear();
        cx.notify();
        true
    }

    /// The row's menu, or with rows picked and this one
    /// among them, Delete N items… for them all. A right-click on a row
    /// that isn't picked lets the others go, so the menu is plainly its.
    pub(crate) fn open_bears_menu(&mut self, key: &str, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        if self.bears_panel.archiving { return; }
        let rows = self.bears_menu_rows(key);
        if rows.is_empty() {
            return;
        }
        self.open_menu(None, rows, position, false, window, cx);
    }

    pub(crate) fn bears_menu_rows(&mut self, key: &str) -> Vec<MenuRow> {
        let kind = if key.starts_with("table:") { Kind::Tables } else { Kind::Bears };
        let picked = self.sidebar_selected(kind);
        if picked.len() > 1 && picked.iter().any(|id| id == key) {
            let keys = picked.clone();
            return vec![
                MenuRow::item(format!("Delete {} items\u{2026}", keys.len()), "trash", MenuAction::Bears(BearsAction::Delete(keys))),
                MenuRow::separator(),
                MenuRow::item("Clear selection", "xmark", MenuAction::Bears(BearsAction::ClearSelection(kind))),
            ];
        }
        if !picked.iter().any(|id| id == key) { self.bears_panel.selected_mut(kind).clear(); }
        let Some(row) = self.bears_panel_rows().into_iter().find(|r| r.key == key) else { return vec![] };
        let mut rows = vec![];
        if row.opens.is_some() {
            rows.push(MenuRow::item("Open", "arrow.right.circle", MenuAction::Bears(BearsAction::Open(key.to_owned()))));
        }
        if key.starts_with("table:") || self.bears_row_folder(key).is_some() {
            rows.push(MenuRow::item(if key.starts_with("bear:") { "Edit Settings\u{2026}" } else { "Edit Table\u{2026}" }, "pencil", MenuAction::Bears(BearsAction::Edit(key.to_owned()))));
        }
        let is_bear = key.starts_with("bear:");
        rows.push(MenuRow::item("Change in a Thread\u{2026}", "bubble.left", MenuAction::Bears(BearsAction::Change(key.to_owned()))));
        rows.push(MenuRow::item(if is_bear { "Tables & Projects\u{2026}" } else { "Bears & Project\u{2026}" }, "person.3", MenuAction::Bears(BearsAction::Seats(key.to_owned()))));
        if !is_bear || self.bears_row_folder(key).is_some() {
            rows.push(MenuRow::item(if self.has_bears_photo(key) { "Change Photo\u{2026}" } else { "Set Photo\u{2026}" }, "photo", MenuAction::Bears(BearsAction::SetPhoto(key.to_owned()))));
            if self.has_bears_photo(key) {
                rows.push(MenuRow::item("Remove Photo", "xmark", MenuAction::Bears(BearsAction::RemovePhoto(key.to_owned()))));
            }
        }
        if self.bears_row_folder(key).is_some() {
            rows.push(MenuRow::item("Show in Finder", "folder", MenuAction::Bears(BearsAction::Reveal(key.to_owned()))));
            rows.push(MenuRow::item("Copy Folder Path", "doc.on.doc", MenuAction::Bears(BearsAction::CopyFolder(key.to_owned()))));
        }
        rows.extend([
            MenuRow::item("Refresh", "arrow.clockwise", MenuAction::Bears(BearsAction::Refresh)),
            MenuRow::separator(),
            MenuRow::item("Rename\u{2026}", "pencil", MenuAction::Bears(BearsAction::Rename(key.to_owned()))),
            MenuRow::item("Delete\u{2026}", "trash", MenuAction::Bears(BearsAction::Delete(vec![key.to_owned()]))),
        ]);
        rows
    }

    pub(crate) fn run_bears_action(&mut self, action: BearsAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            BearsAction::Open(key) => {
                if key.starts_with("table:") { self.bears_panel.tables_view.selecting = false; }
                else { self.bears_panel.bears_view.selecting = false; }
                if let Some(Opens::File(path)) = self.bears_panel_click(&key, Click::Plain, window, cx) {
                    self.open_bears_file(path, false, window, cx);
                }
            }
            BearsAction::Edit(key) => {
                if let Some(id) = key.strip_prefix("table:") {
                    self.open_project_page(id, cx);
                    self.start_editing_project(id, threadbear_core::projects::TextField::Goals, window, cx);
                } else if let Some(folder) = self.bears_row_folder(&key) {
                    self.open_bears_file(format!("{folder}/bear.json"), true, window, cx);
                }
            }
            BearsAction::Reveal(key) => {
                if let Some(folder) = self.bears_row_folder(&key) {
                    crate::open_gate::reveal(&folder, crate::open_gate::Origin::ThisMac, cx);
                }
            }
            BearsAction::CopyFolder(key) => {
                if let Some(folder) = self.bears_row_folder(&key) {
                    cx.write_to_clipboard(ClipboardItem::new_string(folder));
                    self.flash_done("Folder path copied", cx);
                }
            }
            BearsAction::Refresh => {
                self.refresh_bears_panel(true, cx);
                self.flash_done("Refreshing Bears & Tables", cx);
            }
            BearsAction::Rename(key) => self.open_bears_rename(&key, window, cx),
            BearsAction::Delete(keys) => self.open_bears_delete(keys, window, cx),
            BearsAction::ClearSelection(kind) => self.sidebar_clear(kind, cx),
            BearsAction::Change(key) => self.open_bears_sheet(crate::bears_sheets::Mode::Change(key), window, cx),
            BearsAction::Seats(key) => self.open_bears_sheet(crate::bears_sheets::Mode::Seats(key), window, cx),
            BearsAction::SetPhoto(key) => self.choose_bears_photo(&key, window, cx),
            BearsAction::RemovePhoto(key) => self.set_bears_photo(&key, None, cx),
            BearsAction::NewBear => self.open_bears_sheet(crate::bears_sheets::Mode::NewBear, window, cx),
            BearsAction::NewTable => self.open_bears_sheet(crate::bears_sheets::Mode::NewTable, window, cx),
        }
    }

    pub(crate) fn bears_row_folder(&self, key: &str) -> Option<String> {
        let folder = if let Some(id) = key.strip_prefix("bear:") {
            self.bears_panel.bears.as_ref()?.iter().find(|b| b.id == id)?.folder.as_str()
        } else {
            self.project(key.strip_prefix("table:")?)?.folders.first()?.as_str()
        };
        (!folder.is_empty()).then(|| expand_home(folder))
    }

    /// File preview reads the app's settings, so release this app update
    /// before entering the pane. Its editor keeps the usual save safeguards.
    fn open_bears_file(&mut self, path: String, editing: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(thread) = self.pane_thread().filter(|t| !t.is_remote()) else { return };
        let section = self.side.read(cx).section_for(&thread);
        self.show_pane(&section, cx);
        let (side, handle) = (self.side.clone(), window.window_handle());
        cx.defer(move |cx| {
            handle.update(cx, |_, window, cx| {
                side.update(cx, |side, cx| {
                    side.preview_path(&path, None, &thread, cx);
                    if editing && side.previews.get(&thread.id).and_then(|p| p.file_path()) == Some(path.as_str()) {
                        side.set_preview_editing(&thread, true, window, cx);
                    }
                });
            }).ok();
        });
    }

    pub(crate) fn bears_row_name(&self, key: &str) -> Option<String> {
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
        if self.bears_panel.archiving { return; }
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
        self.bears_panel.selection.keys.retain(|k| !keys.contains(k));
        self.bears_panel.table_selection.keys.retain(|k| !keys.contains(k));
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
            .map(|r| json!({ "key": r.key, "section": r.section, "title": r.title, "detail": r.detail, "folder": self.bears_row_folder(&r.key), "picked": self.bears_panel.picked(&r.key) }))
            .collect();
        json!({ "on": self.bears_on(), "tables": self.tables_on(), "loaded": self.bears_panel.bears.is_some(), "rows": rows, "picked": self.bears_panel.selection.keys.iter().chain(self.bears_panel.table_selection.keys.iter()).collect::<Vec<_>>() })
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
        let (loaded, stale) = {
            let a = app.read(cx);
            let state = &a.bears_panel;
            (state.bears.is_some(), a.bears_panel_stale() || state.focus.is_none())
        };
        if stale {
            let app = app.downgrade();
            cx.defer(move |cx| {
                app.update(cx, |a, cx| a.refresh_bears_panel(false, cx)).ok();
            });
        }
        let mut body = v_flex().id("bears-panel").size_full();
        for kind in [Kind::Bears, Kind::Tables] {
            if kind == Kind::Tables && !app.read(cx).tables_on() { continue; }
            let visible = app.read(cx).visible_bears(kind);
            let controls = self.sidebar_controls(kind, cx);
            let mut list = v_flex().py(px(4.));
            if visible.is_empty() { list = list.child(div().px(px(16.)).py(px(8.)).child(if loaded { format!("No {} found", kind.name()) } else { "Loading…".into() }).test_text(if loaded { format!("No {} found", kind.name()) } else { "Loading…".into() })); }
            for row in &visible { list = list.child(self.bears_row(row, app.read(cx).bears_panel.picked(&row.key), true, thread, p, cx)); }
            body = body.child(v_flex().id(SharedString::from(format!("{}-section", kind.name())))
                .flex_1().min_h_0().track_focus(&self.list_focus[kind.index()])
                .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, window, cx| this.sidebar_keys(kind, ev, window, cx)))
                .child(div().px(px(16.)).pt(px(10.)).text_size(px(style::size::SMALL)).font_weight(FontWeight::SEMIBOLD).child(kind.name()).test_text(kind.name()))
                .child(controls)
                .child(div().id(SharedString::from(format!("{}-rows", kind.name()))).flex_1().min_h_0().overflow_y_scroll().child(list)));
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
        let kind = if row.key.starts_with("table:") { Kind::Tables } else { Kind::Bears };
        let selecting = self.app.upgrade().is_some_and(|a| a.read(cx).sidebar_list(kind).selecting);
        let pane_row = PaneRow::new(SharedString::from(format!("bears-row-{}", row.key)), row.title.clone())
            .icon(if selecting { if picked { "checkmark.circle.fill" } else { "circle" }.to_owned() } else { row.icon.clone() })
            .picture(if selecting { None } else { row.picture.clone() })
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
                window.focus(&this.list_focus[kind.index()], cx);
                crate::web_panels::keyboard_to_app(window);
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

}
