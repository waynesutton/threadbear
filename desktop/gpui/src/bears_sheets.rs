// The Bears & Tables panel's sheets (bears_panel.rs), so bears and tables are
// made and changed from the panel itself rather than only with `tb bears`:
//
// - New Bear: a name, what it does, its personality and a photo
//   (`bears.create`, then the photo copied into its folder).
// - New Table: a title, its goal, who sits at it, the project it's a part
//   of and a photo (`tables.create`, then `projects.setIcon`).
// - Tables & Projects (a bear) or Bears & Project (a table): seats
//   (`tables.seat`), the projects a bear's new threads join
//   (`bears.setProjects`) and a table's parent (`projects.update`).
// - Change in a Thread: what to change, handed to a new thread that uses
//   the bears skill, as asking an agent in any thread would.
//
// A bear's photo is a `photo-<ms>.<kind>` file in its own folder
// (core's bears_panel::find_photo), so nothing new goes on the wire; a
// table's is its project icon. Every write waits for the runner, and the
// sheet stays open with the error if it refuses.
use crate::app::Threadbear;
use crate::call;
use crate::spinner::Spinner;
use crate::style::{self, px, size};
use crate::tips::Tip as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, Disableable as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use threadbear_client::runner::Client;
use threadbear_core::bears_panel as rules;
use threadbear_core::projects::Project;

/// What the sheet is for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Mode {
    NewBear,
    NewTable,
    /// A bear's tables and projects, or a table's bears and parent, by row key.
    Seats(String),
    /// A new thread to change this row, by key.
    Change(String),
}

/// A line with a box to tick: a table, a project or a bear.
#[derive(Clone, Debug)]
struct Pick {
    id: String,
    title: String,
    on: bool,
    /// As it was, so only what changed is sent.
    was: bool,
}

pub(crate) struct BearsSheet {
    app: WeakEntity<Threadbear>,
    client: Arc<Client>,
    mode: Mode,
    /// The row's name, for a sheet about one.
    name: String,
    title: Entity<InputState>,
    about: Entity<InputState>,
    text: Entity<TextareaState>,
    /// Tables (for a bear) or bears (for a table or a new one).
    first: Vec<Pick>,
    /// Projects a bear's threads join.
    second: Vec<Pick>,
    /// A table's parent project: the choices, and the one picked.
    parents: Vec<(String, String)>,
    parent: Option<String>,
    parent_was: Option<String>,
    photo: Option<PathBuf>,
    saving: bool,
    error: Option<String>,
}

fn picks(list: impl IntoIterator<Item = (String, String, bool)>) -> Vec<Pick> {
    list.into_iter()
        .map(|(id, title, on)| Pick {
            id,
            title,
            on,
            was: on,
        })
        .collect()
}

impl Threadbear {
    /// Opens one of the panel's sheets.
    pub(crate) fn open_bears_sheet(
        &mut self,
        mode: Mode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.bears_panel.archiving {
            return;
        }
        let bears: Vec<_> = self
            .bears_panel
            .bears
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|b| !b.archived)
            .collect();
        let live: Vec<&Project> = self.projects.list.iter().filter(|p| !p.archived).collect();
        let tables: Vec<&Project> = live.iter().copied().filter(|p| p.table.is_some()).collect();
        let plain: Vec<&Project> = live.iter().copied().filter(|p| p.table.is_none()).collect();
        let key = match &mode {
            Mode::Seats(k) | Mode::Change(k) => Some(k.clone()),
            _ => None,
        };
        let bear = key
            .as_deref()
            .and_then(|k| k.strip_prefix("bear:"))
            .and_then(|id| bears.iter().find(|b| b.id == id).cloned());
        let table = key
            .as_deref()
            .and_then(|k| k.strip_prefix("table:"))
            .and_then(|id| self.project(id).cloned());
        let name = bear
            .as_ref()
            .map(|b| b.name.clone())
            .or_else(|| table.as_ref().map(|t| t.title.clone()))
            .unwrap_or_default();
        if key.is_some() && name.is_empty() {
            return;
        }
        let (mut first, mut second, mut parents, mut parent) = (vec![], vec![], vec![], None);
        match (&mode, &bear, &table) {
            (Mode::Seats(_), Some(b), _) => {
                first = picks(tables.iter().map(|t| {
                    (
                        t.id.clone(),
                        t.title.clone(),
                        t.table.as_ref().is_some_and(|x| x.bears.contains(&b.id)),
                    )
                }));
                second = picks(
                    plain
                        .iter()
                        .map(|p| (p.id.clone(), p.title.clone(), b.projects.contains(&p.id))),
                );
            }
            (Mode::Seats(_), _, Some(t)) => {
                let seated = t
                    .table
                    .as_ref()
                    .map(|x| x.bears.clone())
                    .unwrap_or_default();
                first = picks(
                    bears
                        .iter()
                        .map(|b| (b.id.clone(), b.name.clone(), seated.contains(&b.id))),
                );
                parent = t.parent_id.clone();
            }
            (Mode::NewTable, _, _) => {
                first = picks(bears.iter().map(|b| (b.id.clone(), b.name.clone(), false)))
            }
            _ => {}
        }
        if matches!(mode, Mode::NewTable | Mode::Seats(_)) && bear.is_none() {
            // A part can't have parts, nor a table be its own.
            let me = table.as_ref().map(|t| t.id.clone());
            parents = plain
                .iter()
                .filter(|p| p.parent_id.is_none() && Some(&p.id) != me.as_ref())
                .map(|p| (p.id.clone(), p.title.clone()))
                .collect();
        }
        let client = self.client.clone();
        let app = cx.entity().downgrade();
        let photo_mode = mode.clone();
        let view = cx.new(|cx| {
            let (title_hint, about_hint, text_hint) = match &photo_mode {
                Mode::NewBear => (
                    "Name, like Researcher",
                    "What it does, in a line",
                    "Its personality and how it works (optional)",
                ),
                Mode::NewTable => ("Title, like Launch crew", "Its goal (optional)", ""),
                Mode::Change(_) => (
                    "",
                    "",
                    "What should change? Leave it empty to talk it through in the thread.",
                ),
                Mode::Seats(_) => ("", "", ""),
            };
            BearsSheet {
                app,
                client,
                mode: photo_mode.clone(),
                name: name.clone(),
                title: cx.new(|cx| InputState::new(window, cx).placeholder(title_hint)),
                about: cx.new(|cx| InputState::new(window, cx).placeholder(about_hint)),
                text: cx.new(|cx| {
                    TextareaState::new(window, cx)
                        .placeholder(text_hint)
                        .auto_grow(3, 8)
                }),
                first,
                second,
                parents,
                parent: parent.clone(),
                parent_was: parent,
                photo: None,
                saving: false,
                error: None,
            }
        });
        let focus = match mode {
            Mode::NewBear | Mode::NewTable => Some(view.read(cx).title.read(cx).focus_handle(cx)),
            Mode::Change(_) => Some(view.read(cx).text.read(cx).focus_handle(cx)),
            Mode::Seats(_) => None,
        };
        #[cfg(feature = "automation")]
        OPEN.with(|o| *o.borrow_mut() = Some(view.downgrade()));
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .w(px(460.))
                .close_button(false)
                .child(view.clone())
                .footer(div())
        });
        if let Some(focus) = focus {
            window.focus(&focus, cx);
        }
        crate::web_panels::keyboard_to_app(window);
    }

    /// Change in a Thread: a new thread on the new-thread box's login, model
    /// and mode, told which bear or table and what to change. It opens once
    /// the runner has made it.
    pub(crate) fn start_bears_change_thread(
        &mut self,
        key: &str,
        ask: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = self.bears_row_name(key) else {
            return;
        };
        let folder = self.bears_row_folder(key).unwrap_or_default();
        let prompt = rules::change_prompt(key, &name, &folder, ask);
        let picks = if self.new_thread.picks.runner.is_some() {
            threadbear_core::new_thread::NewThreadPicks::default()
        } else {
            self.new_thread.picks.clone()
        };
        let home = std::env::var("HOME").unwrap_or_default();
        let draft = threadbear_core::new_thread::Draft {
            logins: &self.logins,
            picks: &picks,
            mem: &self.new_thread.prefs.remembered,
            home: &home,
            now_ms: crate::now_ms(),
        };
        let mut params = draft.create_params(&prompt, vec![], false);
        params["title"] = json!(format!("Change {name}"));
        let busy = format!("bearsChange/{key}");
        if !self.begin_note(
            &busy,
            &format!("Starting a thread to change {name}\u{2026}"),
            cx,
        ) {
            return;
        }
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let res = call(client, "threads.create", params).await;
            this.update_in(cx, |t, window, cx| {
                t.end_note(&busy, cx);
                match res.map(|v| {
                    serde_json::from_value::<threadbear_core::inbox::ThreadSummary>(
                        v["thread"].clone(),
                    )
                }) {
                    Ok(Ok(thread)) if !thread.id.is_empty() => {
                        let id = thread.id.clone();
                        t.threads.insert(id.clone(), std::sync::Arc::new(thread));
                        t.open(id, window, cx);
                    }
                    Ok(_) => {
                        t.show_error("The runner didn't say what thread it made.".to_owned(), cx)
                    }
                    Err(e) => t.show_error(e.message, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Set Photo…: a picture from this Mac. A bear's is copied into its
    /// folder; a table's becomes its project icon.
    pub(crate) fn choose_bears_photo(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let picked = crate::picture_picker::choose(window, "Use as Photo", cx);
        let key = key.to_owned();
        cx.spawn_in(window, async move |this, cx| {
            let Some(path) = picked.await else { return };
            this.update(cx, |t, cx| t.set_bears_photo(&key, Some(path), cx))
                .ok();
        })
        .detach();
    }

    /// A photo on, or with none, off (a table goes back to its people icon).
    pub(crate) fn set_bears_photo(
        &mut self,
        key: &str,
        path: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = key.strip_prefix("table:") {
            let mut params = json!({ "projectId": id });
            match &path {
                Some(p) => params["path"] = json!(p.to_string_lossy()),
                None => params["symbol"] = json!("person.3"),
            }
            let client = self.client.clone();
            cx.spawn(async move |this, cx| {
                let res = call(client, "projects.setIcon", params).await;
                this.update(cx, |t, cx| {
                    match res {
                        Ok(v) => {
                            if let Some(project) = Project::decode(&v["project"]) {
                                t.replace_project(project);
                            }
                            t.load_projects(cx);
                        }
                        Err(e) => t.show_error(e.message, cx),
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
            return;
        }
        let Some(id) = key.strip_prefix("bear:").map(str::to_owned) else {
            return;
        };
        let Some(folder) = self.bears_row_folder(key) else {
            return;
        };
        let task = cx
            .background_executor()
            .spawn(async move { place_photo(&folder, path.as_deref()) });
        cx.spawn(async move |this, cx| {
            let res = task.await;
            this.update(cx, |t, cx| {
                match res {
                    Ok(photo) => {
                        if let Some(b) = t
                            .bears_panel
                            .bears
                            .iter_mut()
                            .flatten()
                            .find(|b| b.id == id)
                        {
                            b.photo = photo;
                        }
                    }
                    Err(e) => t.show_error(e, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn has_bears_photo(&self, key: &str) -> bool {
        if let Some(id) = key.strip_prefix("bear:") {
            return self
                .bears_panel
                .bears
                .iter()
                .flatten()
                .any(|b| b.id == id && b.photo.is_some());
        }
        key.strip_prefix("table:")
            .and_then(|id| self.project(id))
            .is_some_and(|p| p.icon.as_ref().is_some_and(|i| i.image_path.is_some()))
    }
}

/// Copies `source` into the bear's folder as its photo, after taking off the
/// ones it had; with none, just takes them off. Gives the photo now there.
fn place_photo(folder: &str, source: Option<&Path>) -> Result<Option<PathBuf>, String> {
    let dir = Path::new(folder);
    let new = match source {
        Some(source) => {
            let name = rules::photo_name(source, crate::now_ms() as i64).ok_or("That isn't a picture Threadbear can use as a photo. Pick a PNG, JPEG, WebP or GIF.")?;
            let meta = std::fs::metadata(source)
                .map_err(|e| format!("Couldn't read that picture: {e}"))?;
            if !meta.is_file() || meta.len() > rules::PHOTO_LIMIT {
                return Err("That picture is too big for a photo. Pick one under 20 MB.".to_owned());
            }
            let to = dir.join(&name);
            std::fs::copy(source, &to)
                .map_err(|e| format!("Couldn't copy the photo into the bear's folder: {e}"))?;
            Some(name)
        }
        None => None,
    };
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("Couldn't open the bear's folder: {e}"))?
        .flatten()
    {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if rules::is_photo_name(&name) && Some(&name) != new.as_ref() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(rules::find_photo(folder))
}

impl BearsSheet {
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
    }

    fn heading(&self) -> String {
        match &self.mode {
            Mode::NewBear => "New Bear".to_owned(),
            Mode::NewTable => "New Table".to_owned(),
            Mode::Seats(k) if k.starts_with("bear:") => {
                format!("{}'s tables and projects", self.name)
            }
            Mode::Seats(_) => format!("Who sits at {}", self.name),
            Mode::Change(_) => format!("Change {}", self.name),
        }
    }

    fn blurb(&self) -> &'static str {
        match &self.mode {
            Mode::NewBear => "A teammate with its own folder, persona and memory. Change anything later from its right-click menu.",
            Mode::NewTable => "A project where bears work together. Its chat threads go in its own folder.",
            Mode::Seats(k) if k.starts_with("bear:") => "Seated at a table, it joins that table's chats. In a project, its new threads join that project.",
            Mode::Seats(_) => "Bears at the table join its chats. A table that's part of a project shows on that project's page.",
            Mode::Change(_) => "Starts a new thread that makes the change with the bears skill, and opens it so you can follow along.",
        }
    }

    fn primary(&self) -> &'static str {
        match &self.mode {
            Mode::NewBear => "Make Bear",
            Mode::NewTable => "Make Table",
            Mode::Seats(_) => "Save",
            Mode::Change(_) => "Start Thread",
        }
    }

    fn can_go(&self, cx: &App) -> bool {
        !self.saving
            && (!matches!(self.mode, Mode::NewBear | Mode::NewTable)
                || !self.title.read(cx).value().trim().is_empty())
    }

    fn choose_photo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = crate::picture_picker::choose(window, "Use as Photo", cx);
        cx.spawn_in(window, async move |this, cx| {
            if let Some(path) = picked.await {
                this.update(cx, |s, cx| {
                    if rules::photo_name(&path, 0).is_some() {
                        s.photo = Some(path);
                        s.error = None;
                    } else {
                        s.error = Some("Pick a PNG, JPEG, WebP or GIF.".to_owned());
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn go(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_go(cx) {
            return;
        }
        let title = self.title.read(cx).value().trim().to_owned();
        let about = self.about.read(cx).value().trim().to_owned();
        let text = self.text.read(cx).value().trim().to_owned();
        if let Mode::Change(key) = &self.mode {
            let key = key.clone();
            window.close_dialog(cx);
            self.app
                .update(cx, |a, cx| {
                    a.start_bears_change_thread(&key, &text, window, cx)
                })
                .ok();
            return;
        }
        let calls = self.calls(&title, &about, &text);
        if calls.is_empty() {
            window.close_dialog(cx);
            return;
        }
        self.saving = true;
        self.error = None;
        cx.notify();
        let (client, mode, photo) = (self.client.clone(), self.mode.clone(), self.photo.clone());
        cx.spawn_in(window, async move |this, cx| {
            let mut answers: Vec<Value> = vec![];
            let mut failed = None;
            // One at a time: each rewrites the runner's bears or projects file.
            for (method, params) in calls {
                match call(client.clone(), method, params).await {
                    Ok(v) => answers.push(v),
                    Err(e) => {
                        failed = Some(e.message);
                        break;
                    }
                }
            }
            this.update_in(cx, |s, window, cx| {
                s.saving = false;
                let app = s.app.clone();
                app.update(cx, |a, cx| {
                    for v in &answers {
                        for k in ["table", "project"] {
                            if let Some(p) = Project::decode(&v[k]) {
                                a.replace_project(p);
                            }
                        }
                    }
                    let made = answers.first().cloned().unwrap_or_default();
                    if failed.is_none() {
                        match (&mode, photo) {
                            (Mode::NewBear, Some(photo)) => {
                                if let Some(id) = made["bear"]["id"].as_str() {
                                    a.pending_bear_photo(
                                        id,
                                        made["bear"]["folder"].as_str().unwrap_or_default(),
                                        photo,
                                        cx,
                                    );
                                }
                            }
                            (Mode::NewTable, Some(photo)) => {
                                if let Some(id) = made["table"]["id"].as_str() {
                                    a.set_bears_photo(&rules::table_key(id), Some(photo), cx);
                                }
                            }
                            _ => {}
                        }
                        let done = match &mode {
                            Mode::NewBear => format!(
                                "Made {}",
                                made["bear"]["name"].as_str().unwrap_or("the bear")
                            ),
                            Mode::NewTable => format!(
                                "Made {}",
                                made["table"]["title"].as_str().unwrap_or("the table")
                            ),
                            _ => "Saved".to_owned(),
                        };
                        a.flash_done(&done, cx);
                    }
                    a.bears_changed();
                    a.refresh_bears_panel(true, cx);
                    a.load_projects(cx);
                    cx.notify();
                })
                .ok();
                match failed {
                    Some(message) => s.error = Some(message),
                    None => window.close_dialog(cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The runner calls the sheet makes, in order. For a new one, the first
    /// call's answer is what was made.
    fn calls(&self, title: &str, about: &str, text: &str) -> Vec<(&'static str, Value)> {
        let on = |list: &[Pick]| {
            list.iter()
                .filter(|p| p.on)
                .map(|p| p.id.clone())
                .collect::<Vec<_>>()
        };
        let added = |list: &[Pick]| {
            list.iter()
                .filter(|p| p.on && !p.was)
                .map(|p| p.id.clone())
                .collect::<Vec<_>>()
        };
        let removed = |list: &[Pick]| {
            list.iter()
                .filter(|p| !p.on && p.was)
                .map(|p| p.id.clone())
                .collect::<Vec<_>>()
        };
        match &self.mode {
            Mode::NewBear => {
                let mut params = json!({ "name": title });
                if !about.is_empty() {
                    params["does"] = json!(about);
                }
                if !text.is_empty() {
                    params["personality"] = json!(text);
                }
                vec![("bears.create", params)]
            }
            Mode::NewTable => {
                let mut params = json!({ "title": title, "bears": on(&self.first) });
                if !about.is_empty() {
                    params["goal"] = json!(about);
                }
                if let Some(parent) = &self.parent {
                    params["parent"] = json!(parent);
                }
                vec![("tables.create", params)]
            }
            Mode::Seats(key) => {
                let mut calls = vec![];
                if let Some(bear) = key.strip_prefix("bear:") {
                    for table in added(&self.first) {
                        calls.push(("tables.seat", json!({ "table": table, "add": [bear] })));
                    }
                    for table in removed(&self.first) {
                        calls.push(("tables.seat", json!({ "table": table, "remove": [bear] })));
                    }
                    let (add, remove) = (added(&self.second), removed(&self.second));
                    if !add.is_empty() || !remove.is_empty() {
                        calls.push((
                            "bears.setProjects",
                            json!({ "bear": bear, "add": add, "remove": remove }),
                        ));
                    }
                } else if let Some(table) = key.strip_prefix("table:") {
                    let (add, remove) = (added(&self.first), removed(&self.first));
                    if !add.is_empty() || !remove.is_empty() {
                        calls.push((
                            "tables.seat",
                            json!({ "table": table, "add": add, "remove": remove }),
                        ));
                    }
                    if self.parent != self.parent_was {
                        calls.push(("projects.update", json!({ "projectId": table, "parentId": self.parent.clone().unwrap_or_default() })));
                    }
                }
                calls
            }
            Mode::Change(_) => vec![],
        }
    }

    fn checklist(
        &self,
        which: usize,
        heading: &'static str,
        empty: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let list = if which == 0 {
            &self.first
        } else {
            &self.second
        };
        let mut col = v_flex().gap(px(4.)).child(
            div()
                .text_size(px(size::LABEL))
                .font_weight(FontWeight::SEMIBOLD)
                .child(heading),
        );
        if list.is_empty() {
            col = col.child(
                div()
                    .text_size(px(size::LABEL))
                    .text_color(style::secondary_text(cx))
                    .child(empty),
            );
        }
        let mut boxes = v_flex()
            .gap(px(4.))
            .max_h(px(160.))
            .id(SharedString::from(format!("bears-sheet-list-{which}")))
            .overflow_y_scroll();
        for (i, pick) in list.iter().enumerate() {
            boxes = boxes.child(
                Checkbox::new(SharedString::from(format!(
                    "bears-sheet-pick-{which}-{}",
                    pick.id
                )))
                .checked(pick.on)
                .label(pick.title.clone())
                .small()
                .disabled(self.saving)
                .on_click(cx.listener(move |s, on: &bool, _, cx| {
                    let list = if which == 0 {
                        &mut s.first
                    } else {
                        &mut s.second
                    };
                    if let Some(p) = list.get_mut(i) {
                        p.on = *on;
                    }
                    cx.notify();
                })),
            );
        }
        col.child(boxes).into_any_element()
    }

    fn parent_menu(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self
            .parent
            .as_ref()
            .and_then(|id| self.parents.iter().find(|(p, _)| p == id))
            .map_or_else(|| "None".to_owned(), |(_, t)| t.clone());
        let mut choices: Vec<(Option<String>, String)> = vec![(None, "None".to_owned())];
        choices.extend(
            self.parents
                .iter()
                .map(|(id, t)| (Some(id.clone()), t.clone())),
        );
        let picked = self.parent.clone();
        let me = cx.entity().downgrade();
        let button = Button::new("bears-sheet-parent")
            .label(current)
            .small()
            .outline()
            .dropdown_caret(true)
            .disabled(self.saving)
            .tooltip("The project this table is a part of, so it shows on that project's page")
            .dropdown_menu(move |menu: PopupMenu, _, _| {
                let mut menu = menu;
                for (id, title) in &choices {
                    let (id, me) = (id.clone(), me.clone());
                    menu = menu.item(
                        PopupMenuItem::new(title.clone())
                            .checked(id == picked)
                            .on_click(move |_, _, cx| {
                                me.update(cx, |s, cx| {
                                    s.parent = id.clone();
                                    cx.notify();
                                })
                                .ok();
                            }),
                    );
                }
                menu
            });
        h_flex()
            .gap(px(10.))
            .items_center()
            .child(
                div()
                    .text_size(px(size::LABEL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Part of project"),
            )
            .child(button)
            .into_any_element()
    }

    fn photo_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let name = self
            .photo
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        let mut row = h_flex().gap(px(10.)).items_center().child(
            div()
                .text_size(px(size::LABEL))
                .font_weight(FontWeight::SEMIBOLD)
                .child("Photo"),
        );
        if let Some(path) = &self.photo {
            row = row.child(
                img(path.clone())
                    .size(px(28.))
                    .rounded(px(6.))
                    .object_fit(ObjectFit::Cover),
            );
        }
        row.child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(size::LABEL))
                .text_color(style::secondary_text(cx))
                .child(name.unwrap_or_else(|| "None yet".to_owned())),
        )
        .child(
            Button::new("bears-sheet-photo")
                .label(if self.photo.is_some() {
                    "Replace\u{2026}"
                } else {
                    "Choose\u{2026}"
                })
                .small()
                .outline()
                .disabled(self.saving)
                .tooltip("Pick a PNG, JPEG, WebP or GIF from your Mac")
                .on_click(cx.listener(|s, _, window, cx| s.choose_photo(window, cx))),
        )
        .when(self.photo.is_some(), |r| {
            r.child(
                Button::new("bears-sheet-photo-clear")
                    .label("Remove")
                    .small()
                    .ghost()
                    .disabled(self.saving)
                    .on_click(cx.listener(|s, _, _, cx| {
                        s.photo = None;
                        cx.notify();
                    })),
            )
        })
        .into_any_element()
    }
}

impl Render for BearsSheet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = v_flex()
            .id("bears-sheet")
            .gap(px(12.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.heading()),
            )
            .child(
                div()
                    .text_size(px(size::LABEL))
                    .text_color(style::secondary_text(cx))
                    .child(self.blurb()),
            );
        match self.mode.clone() {
            Mode::NewBear => {
                body = body
                    .child(
                        div()
                            .id("bears-sheet-name")
                            .tip("Its name, as you'd @mention it")
                            .child(Input::new(&self.title).small()),
                    )
                    .child(
                        div()
                            .id("bears-sheet-does")
                            .tip("Shows under its name in the panel")
                            .child(Input::new(&self.about).small()),
                    )
                    .child(Textarea::new(&self.text))
                    .child(self.photo_row(cx));
            }
            Mode::NewTable => {
                body = body
                    .child(
                        div()
                            .id("bears-sheet-title")
                            .tip("The table's name")
                            .child(Input::new(&self.title).small()),
                    )
                    .child(
                        div()
                            .id("bears-sheet-goal")
                            .tip("What the table is for. It goes in the project's goals")
                            .child(Input::new(&self.about).small()),
                    )
                    .child(self.checklist(
                        0,
                        "Bears at the table",
                        "No bears yet. Make one first with New in the Bears list.",
                        cx,
                    ))
                    .child(self.parent_menu(cx))
                    .child(self.photo_row(cx));
            }
            Mode::Seats(key) if key.starts_with("bear:") => {
                body = body
                    .child(self.checklist(
                        0,
                        "Tables",
                        "No tables yet. Make one with New in the Tables list.",
                        cx,
                    ))
                    .child(self.checklist(
                        1,
                        "Projects its new threads join",
                        "No projects yet.",
                        cx,
                    ));
            }
            Mode::Seats(_) => {
                body = body
                    .child(self.checklist(0, "Bears", "No bears yet.", cx))
                    .child(self.parent_menu(cx));
            }
            Mode::Change(_) => body = body.child(Textarea::new(&self.text)),
        }
        let can = self.can_go(cx);
        body.when_some(self.error.clone(), |b, e| {
            b.child(
                div()
                    .text_size(px(size::LABEL))
                    .text_color(style::failed(cx))
                    .child(e),
            )
        })
        .child(
            h_flex()
                .gap(px(8.))
                .items_center()
                .when(self.saving, |s| {
                    s.child(
                        Spinner::new()
                            .color(style::quiet_text(cx))
                            .with_size(px(12.)),
                    )
                })
                .child(div().flex_1())
                .child(
                    Button::new("bears-sheet-cancel")
                        .label("Cancel")
                        .small()
                        .outline()
                        .on_click(cx.listener(|s, _, window, cx| s.close(window, cx))),
                )
                .child(
                    Button::new("bears-sheet-go")
                        .label(self.primary())
                        .small()
                        .primary()
                        .disabled(!can)
                        .on_click(cx.listener(|s, _, window, cx| s.go(window, cx))),
                ),
        )
    }
}

impl Threadbear {
    /// A new bear's photo, copied in once the runner has made its folder.
    fn pending_bear_photo(
        &mut self,
        id: &str,
        folder: &str,
        photo: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let (id, folder) = (id.to_owned(), folder.to_owned());
        let task = cx
            .background_executor()
            .spawn(async move { place_photo(&folder, Some(&photo)) });
        cx.spawn(async move |this, cx| {
            let res = task.await;
            this.update(cx, |t, cx| {
                match res {
                    Ok(photo) => {
                        if let Some(b) = t
                            .bears_panel
                            .bears
                            .iter_mut()
                            .flatten()
                            .find(|b| b.id == id)
                        {
                            b.photo = photo;
                        }
                        t.refresh_bears_panel(true, cx);
                    }
                    Err(e) => t.show_error(e, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

/// `{"cmd":"bearsSheet", "open": "newBear"|"newTable"|"seats"|"change", "key"?: "<row>"}`
/// opens a sheet; `"photo": "<key>", "path"?: "<file>"` sets (or with no
/// path takes off) a row's photo; `"change": "<key>", "text": "..."` starts
/// the change thread straight away.
#[cfg(feature = "automation")]
thread_local! {
    /// The sheet open last, for a script to fill in and press.
    static OPEN: std::cell::RefCell<Option<WeakEntity<BearsSheet>>> = const { std::cell::RefCell::new(None) };
}

/// On the open sheet: `"fill": { title?, about?, text? }`, `"tick": [ids]`
/// (flips those boxes), `"parent": id|""`, `"photo": path`, `"go": true`;
/// gives back what it holds.
#[cfg(feature = "automation")]
fn drive(cmd: &Value, window: &mut Window, cx: &mut App) -> Result<Value, String> {
    let sheet = OPEN
        .with(|o| o.borrow().clone())
        .and_then(|w| w.upgrade())
        .ok_or("no bears sheet open")?;
    sheet.update(cx, |s, cx| {
        for (k, field) in [("title", s.title.clone()), ("about", s.about.clone())] {
            if let Some(v) = cmd["fill"][k].as_str() {
                field.update(cx, |f, cx| f.set_value(v.to_owned(), window, cx));
            }
        }
        if let Some(v) = cmd["fill"]["text"].as_str() {
            s.text
                .update(cx, |f, cx| f.set_value(v.to_owned(), window, cx));
        }
        for id in cmd["tick"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            for p in s
                .first
                .iter_mut()
                .chain(s.second.iter_mut())
                .filter(|p| p.id == id)
            {
                p.on = !p.on;
            }
        }
        if let Some(p) = cmd["parent"].as_str() {
            s.parent = (!p.is_empty()).then(|| p.to_owned());
        }
        if let Some(p) = cmd["photo"].as_str() {
            s.photo = Some(PathBuf::from(p));
        }
        if cmd["go"].as_bool() == Some(true) {
            s.go(window, cx);
        }
        cx.notify();
    });
    let s = sheet.read(cx);
    let list = |l: &[Pick]| {
        l.iter()
            .map(|p| json!({ "id": p.id, "title": p.title, "on": p.on }))
            .collect::<Vec<_>>()
    };
    Ok(
        json!({ "mode": format!("{:?}", s.mode), "heading": s.heading(), "first": list(&s.first), "second": list(&s.second), "parents": s.parents, "parent": s.parent, "saving": s.saving, "error": s.error }),
    )
}

#[cfg(feature = "automation")]
pub(crate) fn automation(
    app: &Entity<Threadbear>,
    cmd: &Value,
    window: &mut Window,
    cx: &mut App,
) -> Result<Value, String> {
    if cmd.get("fill").is_some()
        || cmd.get("tick").is_some()
        || cmd.get("go").is_some()
        || cmd.get("parent").is_some()
        || cmd["sheet"].as_bool() == Some(true)
        || (cmd.get("photo").is_some()
            && cmd.get("key").is_none()
            && cmd["photo"].as_str().is_some_and(|p| p.starts_with('/')))
    {
        return drive(cmd, window, cx);
    }
    if let Some(key) = cmd["photo"].as_str() {
        let (key, path) = (key.to_owned(), cmd["path"].as_str().map(PathBuf::from));
        app.update(cx, |t, cx| t.set_bears_photo(&key, path, cx));
        return Ok(json!("ok"));
    }
    if let (Some(key), Some(text)) = (cmd["change"].as_str(), cmd["text"].as_str()) {
        let (key, text) = (key.to_owned(), text.to_owned());
        app.update(cx, |t, cx| {
            t.start_bears_change_thread(&key, &text, window, cx)
        });
        return Ok(json!("ok"));
    }
    let key = cmd["key"].as_str().unwrap_or_default().to_owned();
    let mode = match cmd["open"].as_str() {
        Some("newBear") => Mode::NewBear,
        Some("newTable") => Mode::NewTable,
        Some("seats") => Mode::Seats(key),
        Some("change") => Mode::Change(key),
        other => {
            return Err(format!(
                "open must be newBear, newTable, seats or change, not {other:?}"
            ))
        }
    };
    app.update(cx, |t, cx| t.open_bears_sheet(mode, window, cx));
    Ok(json!("ok sheet open"))
}
