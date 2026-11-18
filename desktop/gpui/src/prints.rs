// Prints (docs/protocol.md, "Prints"; "Pages" until 10 Nov 2026, and the
// protocol still says page): the pages agents make for Wayne, kept by the
// runner. A port of the Swift app's Pages.swift, PageSheets.swift and the
// Prints section of ProjectPage.swift: the list (`pages.list`, kept current
// by `pages.updated`), each Print's menu (Open, Show in Finder, Refresh,
// Edit with Agent, Edit Text, Download, Download as PDF, Versions, Projects,
// Publish, Delete), the two sheets those open, and the section on a
// project's page with its Recently deleted. What a Print is and what its
// rows say is threadbear_core::prints. The pages experiment: with it off
// none of this shows.
use crate::app::Threadbear;
use crate::project_page::page_heading;
use crate::sidebar::{MenuAction, MenuRow};
use crate::style::{self, px};
use crate::tips::Tip as _;
use crate::{call, icons, now_ms};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use threadbear_client::runner::{Client, RpcError};
use threadbear_core::prints::{self as core, words, Page};

/// The app's Prints.
#[derive(Default)]
pub(crate) struct PrintsState {
    pub(crate) sidebar: crate::sidebar_lists::ListState,
    /// Every Print but the deleted ones, as the runner last said.
    pub(crate) list: Vec<Page>,
    /// Deleted ones (kept 7 days): asked for when a project's page shows.
    pub(crate) deleted: Vec<Page>,
    /// Prints with a call on the way: their controls show a spinner and
    /// take no clicks (`BusyKey.page`).
    pub(crate) busy: HashSet<String>,
    /// Refresh or Edit with Agent sent at this version, and when; cleared
    /// once a newer version lands (`PageCards.refreshing`).
    pub(crate) refreshing: HashMap<String, (u32, i64)>,
    /// Projects whose Recently deleted is open.
    pub(crate) show_deleted: HashSet<String>,
    /// The runner's Prints folder, learned from any Print's file, so a
    /// click in a todo list is taken only from a Print (`printsRoot`).
    pub(crate) root: Option<String>,
    /// Edit with Agent or Edit Text, while open.
    pub(crate) sheet: Option<Entity<PrintSheet>>,
    /// The row under the pointer on a project's page.
    pub(crate) hovered: Option<String>,
    /// Prints picked with ⌘- or Shift-click, in the order picked, and the
    /// one a Shift-click's run starts from (print_select.rs).
    pub(crate) picked: Vec<String>,
    pub(crate) anchor: Option<String>,
    /// The picked Prints a Delete is asking about.
    pub(crate) asking_delete: Option<Vec<String>>,
}

/// What a Print's menu line does.
#[derive(Clone, Debug)]
pub(crate) enum PrintAction {
    /// In the side pane beside the thread, where it follows new versions.
    OpenHere(String),
    /// In a window of its own, at a version (the newest by default).
    OpenWindow(String, Option<u32>),
    Reveal(String),
    Refresh(String),
    AskAgent(String),
    EditText(String),
    Download(String, Option<u32>),
    DownloadPdf(String, Option<u32>),
    /// That version becomes the newest.
    MakeNewest(String, u32),
    /// Into the project, or out of it.
    ToggleProject(String, String),
    Publish(String),
    Unpublish(String),
    CopyLink(String),
    OpenOnWeb(String),
    Delete(String),
    Undelete(String),
    /// Several picked at once (print_select.rs): Delete asks first.
    DeleteMany(Vec<String>),
    /// Each into the project (true) or out of it.
    ProjectMany(Vec<String>, String, bool),
}

impl Threadbear {
    /// Prints are on for this runner (hello lists the pages experiment).
    pub(crate) fn prints_on(&self) -> bool {
        self.remote.experiments.iter().any(|e| e == core::EXPERIMENT)
    }

    /// The rail's Prints button (`showsPrintsRail`): Prints on, at least one
    /// Print, and a thread of this runner's, not a project's page.
    pub(crate) fn shows_prints_rail(&self, thread: &threadbear_core::inbox::ThreadSummary) -> bool {
        self.prints_on() && self.projects.open.is_none() && !thread.is_remote() && self.prints.list.iter().any(|p| p.deleted_at.is_none())
    }

    pub(crate) fn print(&self, id: &str) -> Option<&Page> {
        self.prints.list.iter().find(|p| p.id == id).or_else(|| self.prints.deleted.iter().find(|p| p.id == id))
    }

    /// The list after hello; `pages.updated` keeps it current from then on.
    pub(crate) fn load_prints(&mut self, cx: &mut Context<Self>) {
        if !self.prints_on() {
            self.prints.list.clear();
            return;
        }
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = call(client, "pages.list", json!({})).await;
            this.update(cx, |this, cx| {
                // An older runner, or the experiment just turned off: none.
                let pages = res.map(|v| Page::list(&v["pages"])).unwrap_or_default();
                this.receive_prints(pages, cx);
            })
            .ok();
        })
        .detach();
    }

    /// Deleted ones aren't in `pages.updated`: asked for when a project's
    /// page shows, and after a delete or restore.
    pub(crate) fn load_deleted_prints(&mut self, cx: &mut Context<Self>) {
        if !self.prints_on() {
            return;
        }
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = call(client, "pages.list", json!({ "deleted": true })).await;
            this.update(cx, |this, cx| {
                this.prints.deleted = res.map(|v| Page::list(&v["pages"])).unwrap_or_default();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// A new list, from `pages.list` or `pages.updated`: a side pane showing
    /// a Print's newest version moves to its new one, and a card's
    /// "refreshing" ends once a newer version is in.
    pub(crate) fn receive_prints(&mut self, pages: Vec<Page>, cx: &mut Context<Self>) {
        let before = std::mem::replace(&mut self.prints.list, pages);
        let after = self.prints.list.clone();
        self.prints.refreshing.retain(|id, (v, _)| after.iter().find(|p| &p.id == id).is_none_or(|p| p.version <= *v));
        if before != after {
            let side = self.side.clone();
            cx.defer(move |cx| side.update(cx, |side, cx| side.follow_print_versions(&before, &after, cx)));
        }
        crate::print_cards::changed(cx);
        cx.notify();
    }

    fn replace_print(&mut self, page: Page, cx: &mut Context<Self>) {
        let mut list = self.prints.list.clone();
        match list.iter_mut().find(|p| p.id == page.id) {
            Some(slot) => *slot = page,
            None if page.deleted_at.is_none() => list.insert(0, page),
            None => {}
        }
        self.receive_prints(list, cx);
    }

    pub(crate) fn print_busy(&self, id: &str) -> bool {
        self.prints.busy.contains(id)
    }

    /// `isRefreshing`: Refresh or Edit with Agent sent, and no newer version
    /// yet, in the last 20 minutes.
    pub(crate) fn print_refreshing(&self, page: &Page) -> bool {
        self.prints.refreshing.get(&page.id).is_some_and(|(v, at)| page.version <= *v && now_ms() - at < core::REFRESH_FOR_MS)
    }

    /// A call about one Print, while its controls show it's under way and
    /// take no second click; `note` says so in the banner for a click from
    /// a menu. `done` gets the answer; an error shows in the error bar.
    fn print_call(&mut self, id: &str, note: Option<String>, method: &'static str, params: Value, cx: &mut Context<Self>, done: impl FnOnce(&mut Self, Value, &mut Context<Self>) + 'static) {
        if !self.prints.busy.insert(id.to_owned()) {
            return;
        }
        let key = format!("print:{id}");
        if let Some(note) = &note {
            self.begin_note(&key, note, cx);
        }
        crate::print_cards::changed(cx);
        cx.notify();
        let (client, id) = (self.client.clone(), id.to_owned());
        cx.spawn(async move |this, cx| {
            let res = call(client, method, params).await;
            this.update(cx, |this, cx| {
                this.prints.busy.remove(&id);
                this.end_note(&key, cx);
                match res {
                    Ok(v) => done(this, v, cx),
                    Err(e) => this.show_error(print_error(&e), cx),
                }
                crate::print_cards::changed(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// A click in a todo list or board (`threadbear-print:`), from the page
    /// `file` shows (`handlePrintLink`). Only a page in the runner's own
    /// Prints folder is listened to, so another local page can't tick a
    /// Print by carrying the same link. The side pane follows the new
    /// version by itself (`follow_print_versions`).
    pub(crate) fn handle_print_link(&mut self, url: &str, file: &str, cx: &mut Context<Self>) {
        let Some(link) = core::PrintLink::parse(url) else { return };
        let real = |p: &str| std::fs::canonicalize(p).ok();
        let Some(root) = self.prints.root.as_deref().and_then(real) else { return };
        if !real(file).is_some_and(|f| f.starts_with(&root)) {
            return;
        }
        let Some((id, _shown)) = core::path::parse(file) else { return };
        let Some(page) = self.print(&id).cloned() else { return };
        let (method, params) = link.call(&id);
        let note = page.is_list().then(|| format!("Saving {}", page.title));
        self.print_call(&id, note, method, params, cx, |this, v, cx| {
            if let Some(page) = Page::decode(&v["page"]) {
                this.replace_print(page, cx);
            }
        });
    }

    /// Notes the runner's Prints folder from a file it handed over.
    pub(crate) fn note_prints_root(&mut self, file: &str) {
        if let Some(root) = core::path::root(file) {
            self.prints.root = Some(root);
        }
    }

    // ---- The menu ----

    /// The same lines on a project page's row, a reply's card, a Highlights
    /// row and the rail's list (`PageMenu`); `here` puts Open Beside the
    /// Thread first, where there's a thread to be beside.
    pub(crate) fn print_menu_rows(&self, page: &Page, here: bool) -> Vec<MenuRow> {
        let id = page.id.clone();
        let act = |a: PrintAction| MenuAction::Print(a);
        let mut rows = Vec::new();
        if here {
            rows.push(MenuRow::item("Open Beside the Thread", "sidebar.right", act(PrintAction::OpenHere(id.clone()))));
        }
        rows.push(MenuRow::item(if here { "Open in a Window" } else { "Open" }, "macwindow", act(PrintAction::OpenWindow(id.clone(), None))));
        rows.push(MenuRow::item("Show in Finder", "folder", act(PrintAction::Reveal(id.clone()))));
        rows.push(MenuRow::item("Refresh", "arrow.clockwise", act(PrintAction::Refresh(id.clone()))));
        rows.push(MenuRow::item("Edit with Agent…", "sparkles", act(PrintAction::AskAgent(id.clone()))));
        if page.has_editable_source() {
            rows.push(MenuRow::item("Edit Text…", "pencil", act(PrintAction::EditText(id.clone()))));
        }
        rows.push(MenuRow::separator());
        rows.push(MenuRow::item("Download…", "arrow.down.circle", act(PrintAction::Download(id.clone(), None))));
        rows.push(MenuRow::item("Download as PDF…", "doc.richtext", act(PrintAction::DownloadPdf(id.clone(), None))));
        let versions = page
            .kept_versions()
            .into_iter()
            .map(|v| {
                if v == page.version {
                    MenuRow::item(format!("Open Version {v} (newest)"), "macwindow", act(PrintAction::OpenWindow(id.clone(), Some(v))))
                } else {
                    MenuRow::submenu(
                        format!("Version {v}"),
                        "doc",
                        vec![
                            MenuRow::item("Open", "macwindow", act(PrintAction::OpenWindow(id.clone(), Some(v)))),
                            MenuRow::item("Make This the Newest", "arrow.uturn.backward", act(PrintAction::MakeNewest(id.clone(), v))),
                            MenuRow::item("Download…", "arrow.down.circle", act(PrintAction::Download(id.clone(), Some(v)))),
                            MenuRow::item("Download as PDF…", "doc.richtext", act(PrintAction::DownloadPdf(id.clone(), Some(v)))),
                        ],
                    )
                }
            })
            .collect();
        rows.push(MenuRow::submenu("Versions", "clock.arrow.circlepath", versions));
        let projects: Vec<MenuRow> = self.projects.list.iter().filter(|p| !p.archived).map(|p| MenuRow::check(p.title.clone(), "square.stack", page.is_in(&p.id), act(PrintAction::ToggleProject(id.clone(), p.id.clone())))).collect();
        if !projects.is_empty() {
            rows.push(MenuRow::submenu("Projects", "square.stack", projects));
        }
        rows.push(MenuRow::separator());
        if page.published_slug.is_some() {
            rows.push(MenuRow::item("Copy Link", "link", act(PrintAction::CopyLink(id.clone()))));
            if self.print_link(page).is_some() {
                rows.push(MenuRow::item("Open on the Web", "safari", act(PrintAction::OpenOnWeb(id.clone()))));
            }
            rows.push(MenuRow::item("Unpublish", "eye.slash", act(PrintAction::Unpublish(id.clone()))));
        } else if !page.live {
            rows.push(MenuRow::item("Publish…", "globe", act(PrintAction::Publish(id.clone()))));
        }
        rows.push(MenuRow::separator());
        rows.push(MenuRow::item("Delete", "trash", act(PrintAction::Delete(id))));
        rows
    }

    pub(crate) fn open_print_menu(&mut self, id: &str, here: bool, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.print(id).cloned() else { return };
        let rows = self.print_menu_rows(&page, here);
        self.open_menu(None, rows, position, false, window, cx);
    }

    pub(crate) fn run_print_action(&mut self, action: PrintAction, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            PrintAction::OpenHere(id) => self.open_print_here(&id, window, cx),
            PrintAction::OpenWindow(id, version) => self.open_print_window(&id, version, cx),
            PrintAction::Reveal(id) => self.reveal_print(&id, cx),
            PrintAction::Refresh(id) => self.refresh_print(&id, cx),
            PrintAction::AskAgent(id) => self.open_print_sheet(&id, SheetKind::Ask, window, cx),
            PrintAction::EditText(id) => self.open_print_sheet(&id, SheetKind::Edit, window, cx),
            PrintAction::Download(id, version) => self.download_print(&id, version, cx),
            PrintAction::DownloadPdf(id, version) => self.download_print_pdf(&id, version, cx),
            PrintAction::MakeNewest(id, version) => {
                let title = self.print(&id).map(|p| p.title.clone()).unwrap_or_default();
                self.print_call(&id, Some(format!("Restoring version {version} of {title}")), "pages.restoreVersion", json!({ "pageId": id, "version": version }), cx, |this, v, cx| {
                    if let Some(page) = Page::decode(&v["page"]) {
                        this.replace_print(page, cx);
                    }
                });
            }
            PrintAction::ToggleProject(id, project) => {
                let adding = !self.print(&id).is_some_and(|p| p.is_in(&project));
                let params = if adding { json!({ "pageId": id, "add": [project] }) } else { json!({ "pageId": id, "remove": [project] }) };
                self.print_call(&id, None, "pages.setProjects", params, cx, |this, v, cx| {
                    if let Some(page) = Page::decode(&v["page"]) {
                        this.replace_print(page, cx);
                    }
                });
            }
            PrintAction::Publish(id) => self.publish_print(&id, cx),
            PrintAction::Unpublish(id) => {
                let title = self.print(&id).map(|p| p.title.clone()).unwrap_or_default();
                self.print_call(&id, Some(format!("Unpublishing {title}")), "pages.unpublish", json!({ "pageId": id }), cx, |this, v, cx| {
                    if let Some(page) = Page::decode(&v["page"]) {
                        this.replace_print(page, cx);
                    }
                });
            }
            PrintAction::CopyLink(id) => {
                if let Some(link) = self.print(&id).and_then(|p| self.print_link(p)) {
                    cx.write_to_clipboard(ClipboardItem::new_string(link));
                    self.flash_done("Link copied", cx);
                }
            }
            PrintAction::OpenOnWeb(id) => {
                if let Some(link) = self.print(&id).and_then(|p| self.print_link(p)) {
                    crate::open_gate::open_web(&link, cx);
                }
            }
            PrintAction::Delete(id) => self.delete_print(&id, cx),
            PrintAction::Undelete(id) => self.undelete_print(&id, cx),
            PrintAction::DeleteMany(ids) => self.ask_delete_prints(ids, window, cx),
            PrintAction::ProjectMany(ids, project, add) => self.set_prints_project(ids, project, add, cx),
        }
    }

    // ---- What each does ----

    /// In a window of its own, the way a reply's page opens: sealed, its
    /// scripts off (rich_window.rs). The window shows the page's HTML.
    pub(crate) fn open_print_window(&mut self, id: &str, version: Option<u32>, cx: &mut Context<Self>) {
        let mut params = json!({ "pageId": id });
        if let Some(v) = version {
            params["version"] = json!(v);
        }
        self.print_call(id, None, "pages.get", params, cx, move |this, v, cx| {
            let Some(file) = v["file"].as_str().map(str::to_owned) else { return };
            this.note_prints_root(&file);
            cx.spawn(async move |this, cx| {
                let read = cx.background_executor().spawn(async move { std::fs::read_to_string(&file) }).await;
                this.update(cx, |this, cx| match read {
                    Ok(html) => {
                        let block = threadbear_core::rich_block::RichBlock::new(threadbear_core::rich_block::RichKind::Html, html);
                        if let Some(window) = cx.active_window().or_else(|| crate::window_menu::main_window(cx)) {
                            window.update(cx, |_, window, cx| crate::rich_window::open(&block, window, cx)).ok();
                        }
                    }
                    Err(e) => this.show_error(format!("Couldn't read the print: {e}"), cx),
                })
                .ok();
            })
            .detach();
        });
    }

    /// From a reply's card, Highlights or the rail: in the thread's side
    /// pane, as a highlight opens, where it follows new versions. With no
    /// thread of this Mac's to be beside, it gets a window instead.
    pub(crate) fn open_print_here(&mut self, id: &str, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(thread) = self.pane_thread().filter(|t| !t.is_remote()) else { return self.open_print_window(id, None, cx) };
        let title = self.print(id).map(|p| p.title.clone());
        self.print_call(id, None, "pages.get", json!({ "pageId": id }), cx, move |this, v, cx| {
            let Some(file) = v["file"].as_str().map(str::to_owned) else { return };
            this.note_prints_root(&file);
            let title = title.or_else(|| v["page"]["title"].as_str().map(str::to_owned));
            // Once this update is over: the pane reads the app to know its
            // thread, which GPUI refuses while the app is being updated.
            let (side, app) = (this.side.clone(), cx.entity());
            cx.defer(move |cx| {
                let section = side.update(cx, |side, cx| {
                    side.preview_path(&file, title, &thread, cx);
                    side.section_for(&thread)
                });
                app.update(cx, |this, cx| {
                    if !this.dock.read(cx).controller.is_showing(&section) {
                        this.show_pane(&section, cx);
                    }
                });
            });
        });
    }

    /// Its newest index.html, in its version's folder, in the Finder.
    fn reveal_print(&mut self, id: &str, cx: &mut Context<Self>) {
        self.print_call(id, None, "pages.get", json!({ "pageId": id }), cx, |this, v, cx| {
            if let Some(file) = v["file"].as_str() {
                this.note_prints_root(file);
                crate::open_gate::reveal(file, crate::open_gate::Origin::ThisMac, cx);
            }
        });
    }

    /// Sends it to the thread that made it to be made again from its
    /// recipe, and opens that thread so its work can be watched.
    fn refresh_print(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(page) = self.print(id).cloned() else { return };
        self.prints.refreshing.insert(page.id.clone(), (page.version, now_ms()));
        self.print_call(id, Some(format!("Asking a thread to refresh {}", page.title)), "pages.refresh", json!({ "pageId": id }), cx, |this, v, cx| {
            if let Some(thread) = v["threadId"].as_str() {
                this.open_print_thread(thread.to_owned(), cx);
            }
        });
    }

    pub(crate) fn edit_print_with_agent(&mut self, id: &str, text: String, cx: &mut Context<Self>) {
        let Some(page) = self.print(id).cloned() else { return };
        if text.trim().is_empty() {
            return;
        }
        self.prints.refreshing.insert(page.id.clone(), (page.version, now_ms()));
        self.print_call(id, Some(format!("Asking a thread to change {}", page.title)), "pages.edit", json!({ "pageId": id, "text": text }), cx, |this, v, cx| {
            if let Some(thread) = v["threadId"].as_str() {
                this.open_print_thread(thread.to_owned(), cx);
            }
        });
    }

    fn open_print_thread(&mut self, thread: String, cx: &mut Context<Self>) {
        let Some(window) = crate::window_menu::main_window(cx) else { return };
        let entity = cx.entity();
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    entity.update(cx, |this, cx| {
                        this.projects.open = None;
                        this.reveal_thread(&thread, cx);
                        this.open(thread, window, cx);
                    })
                })
                .ok();
        });
    }

    /// Asks where, copies the Print's files into a new folder named for it
    /// there, and shows it in the Finder.
    fn download_print(&mut self, id: &str, version: Option<u32>, cx: &mut Context<Self>) {
        let Some(page) = self.print(id).cloned() else { return };
        let paths = cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Save Here".into()) });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(chosen))) = paths.await else { return };
            let Some(folder) = chosen.into_iter().next() else { return };
            let name = page.file_name();
            let mut dest = folder.join(&name);
            let mut n = 2;
            while dest.exists() {
                dest = folder.join(format!("{name} {n}"));
                n += 1;
            }
            let mut params = json!({ "pageId": page.id, "dest": dest.to_string_lossy() });
            if let Some(v) = version {
                params["version"] = json!(v);
            }
            this.update(cx, |this, cx| {
                this.print_call(&page.id, Some(format!("Saving {}", page.title)), "pages.export", params, cx, |_, v, cx| {
                    if let Some(file) = v["file"].as_str() {
                        crate::open_gate::reveal(file, crate::open_gate::Origin::ThisMac, cx);
                    }
                });
            })
            .ok();
        })
        .detach();
    }

    /// Asks where, draws the Print (or a version) as a US Letter PDF in the
    /// light look, saves it, and shows it in the Finder.
    fn download_print_pdf(&mut self, id: &str, version: Option<u32>, cx: &mut Context<Self>) {
        let Some(page) = self.print(id).cloned() else { return };
        let home = crate::files::home_dir();
        let suggested = match version {
            Some(v) => format!("{} v{v}.pdf", page.file_name()),
            None => format!("{}.pdf", page.file_name()),
        };
        let path = cx.prompt_for_new_path(std::path::Path::new(&home).join("Downloads").as_path(), Some(&suggested));
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(dest))) = path.await else { return };
            let mut params = json!({ "pageId": page.id });
            if let Some(v) = version {
                params["version"] = json!(v);
            }
            this.update(cx, |this, cx| {
                this.print_call(&page.id.clone(), Some(format!("Saving {} as a PDF", page.title)), "pages.get", params, cx, move |this, v, cx| {
                    let Some(file) = v["file"].as_str().map(str::to_owned) else { return };
                    this.save_print_pdf(file, dest, page.title.clone(), cx);
                });
            })
            .ok();
        })
        .detach();
    }

    /// Draws the page in `file` as a PDF at `dest` and shows it in the
    /// Finder. It needs the web runtime, as a reply's pages do.
    fn save_print_pdf(&mut self, file: String, dest: std::path::PathBuf, title: String, cx: &mut Context<Self>) {
        let Some(runtime) = crate::web_runtime::runtime(cx) else { return self.show_error("Saving a PDF needs the web runtime, which this app can't use here.".into(), cx) };
        if runtime.read(cx).state != threadbear_core::web_runtime::State::Ready {
            return self.show_error("Saving a PDF needs the web runtime. Install it from Settings > General, then try again.".into(), cx);
        }
        let key = format!("pdf:{}", dest.display());
        self.begin_note(&key, &format!("Saving {title} as a PDF"), cx);
        cx.spawn(async move |this, cx| {
            // Chromium started, if it isn't yet.
            loop {
                let up = runtime.update(cx, |r, cx| (r.ensure_engine(cx), matches!(r.engine, crate::web_runtime::Engine::Failed(_))));
                if up.0 || up.1 {
                    break;
                }
                cx.background_executor().timer(std::time::Duration::from_millis(100)).await;
            }
            let read = cx.background_executor().spawn(async move { std::fs::read_to_string(&file) }).await;
            let path = dest.to_string_lossy().into_owned();
            let saved = match read {
                Ok(html) => threadbear_web::page_pdf(&html, &path).await,
                Err(e) => Err(e.to_string()),
            };
            this.update(cx, |this, cx| {
                this.end_note(&key, cx);
                match saved {
                    Ok(()) => crate::open_gate::reveal(&path, crate::open_gate::Origin::ThisMac, cx),
                    Err(why) => this.show_error(format!("Couldn't save the PDF: {why}"), cx),
                }
            })
            .ok();
        })
        .detach();
    }

    /// The runner's address for a published Print (`printLink`).
    pub(crate) fn print_link(&self, page: &Page) -> Option<String> {
        let slug = page.published_slug.as_ref()?;
        let site = self.website_url.as_deref()?.trim_end_matches('/');
        Some(format!("{site}/p/{slug}"))
    }

    /// Asks first (anyone with the link can open it), then sends it to the
    /// Convex backend and copies its link.
    fn publish_print(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(page) = self.print(id).cloned() else { return };
        let title = format!("Publish \u{201C}{}\u{201D}?", page.title);
        cx.spawn(async move |this, cx| {
            if !crate::alert::confirm(&title, words::PUBLISH_ABOUT, "Publish", "Cancel") {
                return;
            }
            this.update(cx, |this, cx| {
                this.print_call(&page.id, Some(format!("Publishing {}", page.title)), "pages.publish", json!({ "pageId": page.id }), cx, |this, v, cx| {
                    if let Some(page) = Page::decode(&v["page"]) {
                        let link = v["url"].as_str().map(str::to_owned).or_else(|| this.print_link(&page));
                        this.replace_print(page, cx);
                        if let Some(link) = link {
                            cx.write_to_clipboard(ClipboardItem::new_string(link));
                            this.flash_done("Published, and its link copied", cx);
                        }
                    }
                });
            })
            .ok();
        })
        .detach();
    }

    /// Takes the row away at once; the runner keeps it 7 days.
    fn delete_print(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(page) = self.print(id).cloned() else { return };
        let before = self.prints.list.clone();
        self.prints.list.retain(|p| p.id != page.id);
        crate::print_cards::changed(cx);
        cx.notify();
        let (client, id) = (self.client.clone(), page.id.clone());
        self.prints.busy.insert(id.clone());
        cx.spawn(async move |this, cx| {
            let res = call(client, "pages.delete", json!({ "pageId": id })).await;
            this.update(cx, |this, cx| {
                this.prints.busy.remove(&id);
                match res {
                    Ok(v) => {
                        if let Some(gone) = Page::decode(&v["page"]) {
                            this.prints.deleted.retain(|p| p.id != gone.id);
                            this.prints.deleted.insert(0, gone);
                        }
                    }
                    Err(e) => {
                        this.receive_prints(before, cx);
                        this.show_error(print_error(&e), cx);
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn undelete_print(&mut self, id: &str, cx: &mut Context<Self>) {
        self.prints.deleted.retain(|p| p.id != id);
        cx.notify();
        let (client, id) = (self.client.clone(), id.to_owned());
        cx.spawn(async move |this, cx| {
            let res = call(client, "pages.restore", json!({ "pageId": id })).await;
            this.update(cx, |this, cx| match res {
                Ok(v) => {
                    if let Some(page) = Page::decode(&v["page"]) {
                        this.replace_print(page, cx);
                    }
                }
                Err(e) => {
                    this.load_deleted_prints(cx);
                    this.show_error(print_error(&e), cx);
                }
            })
            .ok();
        })
        .detach();
    }

    // ---- The sheets ----

    pub(crate) fn open_print_sheet(&mut self, id: &str, kind: SheetKind, window: &mut Window, cx: &mut Context<Self>) {
        // A menu left open goes with it.
        self.sidebar.menu = None;
        let Some(page) = self.print(id).cloned() else { return };
        let app = cx.entity().downgrade();
        let client = self.client.clone();
        let view = cx.new(|cx| PrintSheet::new(app.clone(), client, page, kind, window, cx));
        self.prints.sheet = Some(view.clone());
        cx.notify();
        let field = view.read(cx).text.clone();
        let width = if kind == SheetKind::Edit { 720. } else { 480. };
        window.open_dialog(cx, move |dialog, _, _| {
            let app = app.clone();
            dialog.w(px(width)).close_button(false).child(view.clone()).footer(div()).on_close(move |_, _, cx| {
                if let Some(app) = app.upgrade() {
                    app.update(cx, |app, cx| {
                        app.prints.sheet = None;
                        cx.notify();
                    });
                }
            })
        });
        window.defer(cx, move |window, cx| {
            field.update(cx, |f, cx| f.focus(window, cx));
            crate::web_panels::keyboard_to_app(window);
        });
    }

    // ---- A project's page ----

    /// `PagesSection`: its Prints, newest first, each opening in a window
    /// of its own with its menu at the right; deleted ones under Recently
    /// deleted for 7 days.
    pub(crate) fn render_prints_section(&mut self, project_id: &str, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.prints_on() {
            return None;
        }
        let pages = core::rows::in_project(project_id, &self.prints.list);
        let deleted: Vec<Page> = self.prints.deleted.iter().filter(|p| p.is_in(project_id)).cloned().collect();
        let (faint, secondary) = (style::faint_text(cx), style::secondary_text(cx));
        let mut section = v_flex().w_full().id("project-prints").child(page_heading("Prints", (!pages.is_empty()).then(|| pages.len().to_string()), None, cx));
        if pages.is_empty() {
            section = section.child(div().text_size(px(style::size::LABEL)).line_height(px(16.)).text_color(faint).child(words::SECTION_EMPTY));
        }
        let now = now_ms();
        // Picked ones in this list: the bar over it says how many.
        let order: std::rc::Rc<Vec<String>> = std::rc::Rc::new(pages.iter().map(|p| p.id.clone()).collect());
        let picked_here = self.picked_prints().iter().filter(|id| order.contains(id)).count();
        section = section.children(crate::print_select::picked_bar(cx.entity().downgrade(), picked_here, cx));
        let mut rows = v_flex().w_full().gap(px(2.));
        for page in &pages {
            rows = rows.child(self.render_print_row(page, now, order.clone(), cx));
        }
        section = section.child(rows);
        if !deleted.is_empty() {
            let open = self.prints.show_deleted.contains(project_id);
            let key = project_id.to_owned();
            section = section.child(
                h_flex()
                    .id("prints-deleted")
                    .role(Role::Button)
                    .pt(px(8.))
                    .gap(px(4.))
                    .items_center()
                    .cursor_pointer()
                    .text_size(px(style::size::DETAIL))
                    .text_color(faint)
                    .tip("Prints deleted in the last 7 days. Restore brings one back")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if !this.prints.show_deleted.remove(&key) {
                            this.prints.show_deleted.insert(key.clone());
                        }
                        cx.notify();
                    }))
                    .child(icons::symbol(if open { "chevron.down" } else { "chevron.right" }).with_size(px(10.)))
                    .child(format!("Recently deleted ({})", deleted.len())),
            );
            if open {
                let mut list = v_flex().pt(px(6.)).gap(px(4.));
                for page in &deleted {
                    let id = page.id.clone();
                    let when = page.deleted_at.map(|at| format!("deleted {}", threadbear_core::inbox::ago(at, now))).unwrap_or_default();
                    let busy = self.print_busy(&page.id);
                    list = list.child(
                        h_flex()
                            .gap(px(8.))
                            .items_center()
                            .text_size(px(style::size::LABEL))
                            .child(icons::symbol(page.symbol()).with_size(px(12.)).text_color(faint))
                            .child(div().child(page.title.clone()))
                            .child(div().text_size(px(style::size::DETAIL)).text_color(faint).child(when))
                            .child(div().flex_1())
                            .child(if busy {
                                Spinner::new().color(secondary).with_size(px(11.)).into_any_element()
                            } else {
                                crate::project_page::small_push_button(SharedString::from(format!("print-restore-{}", page.id)), "Restore", "Bring this print back. Deleted prints are kept for 7 days", cx, cx.listener(move |this, _: &ClickEvent, window, cx| this.run_print_action(PrintAction::Undelete(id.clone()), window, cx)))
                            }),
                    );
                }
                section = section.child(list);
            }
        }
        Some(section.into_any_element())
    }

    fn render_print_row(&mut self, page: &Page, now: i64, order: std::rc::Rc<Vec<String>>, cx: &mut Context<Self>) -> AnyElement {
        let (faint, fg) = (style::faint_text(cx), cx.theme().foreground);
        let key = format!("print:{}", page.id);
        let hovering = self.prints.hovered.as_deref() == Some(key.as_str());
        let picked = self.print_picked(&page.id);
        let busy = self.print_busy(&page.id);
        let (open_id, menu_id, more_id) = (page.id.clone(), page.id.clone(), page.id.clone());
        let hover_key = key.clone();
        let more = div()
            .id(SharedString::from(format!("print-more-{}", page.id)))
            .role(Role::Button)
            .test_support()
            .flex_none()
            .size(px(22.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.))
            .cursor_pointer()
            .opacity(if hovering || busy { 1. } else { 0.5 })
            .hover(|s| s.bg(style::sidebar_hover(cx)))
            .tip("Refresh, edit, download, versions, move or delete this print")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, ev: &ClickEvent, window, cx| {
                let at = ev.position();
                this.open_print_menu(&more_id, false, point(at.x, at.y + px(12.)), window, cx);
            }))
            .child(if busy { Spinner::new().color(style::secondary_text(cx)).with_size(px(11.)).into_any_element() } else { icons::symbol("ellipsis").with_size(px(13.)).text_color(style::secondary_text(cx)).into_any_element() });
        h_flex()
            .id(SharedString::from(format!("print-row-{}", page.id)))
            .role(Role::Button)
            .test_support()
            .w_full()
            .items_start()
            .gap(px(10.))
            .px(px(8.))
            .mx(px(-8.))
            .py(px(6.))
            .rounded(px(6.))
            .cursor_pointer()
            .when(hovering && !picked, |s| s.bg(style::sidebar_hover(cx)))
            .when(picked, |s| s.bg(style::primary_opacity(cx, 0.1)).aria_selected(true))
            .tip("Open this print in a window of its own. \u{2318}-click or Shift-click picks several")
            .on_hover(cx.listener(move |this, on: &bool, _, cx| {
                let now = if *on { Some(hover_key.clone()) } else { None };
                if *on || this.prints.hovered.as_deref() == Some(hover_key.as_str()) {
                    this.prints.hovered = now;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                if this.print_row_clicked(&open_id, &order, crate::print_select::Pick::from(&ev.modifiers()), cx) {
                    return;
                }
                if !this.print_busy(&open_id) {
                    this.open_print_window(&open_id, None, cx);
                }
            }))
            .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev: &MouseDownEvent, window, cx| this.print_row_menu(&menu_id, false, ev.position, window, cx)))
            .child(div().pt(px(2.)).w(px(16.)).flex_none().child(icons::symbol(page.symbol()).with_size(px(13.)).text_color(faint)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        h_flex()
                            .gap(px(6.))
                            .items_baseline()
                            .child(div().flex_shrink(1.).min_w_0().truncate().text_size(px(style::size::LABEL)).line_height(px(16.)).font_medium().text_color(fg).child(page.title.clone()))
                            .child(div().flex_none().text_size(px(style::size::DETAIL)).line_height(px(15.)).text_color(faint).child(page.row_detail(now))),
                    )
                    .when(!page.summary.is_empty(), |s| s.child(div().text_size(px(style::size::DETAIL)).line_height(px(15.)).text_color(faint).line_clamp(2).child(page.summary.clone()))),
            )
            .child(more)
            .into_any_element()
    }
}

/// The runner's words, without "pages." when it names a method.
pub(crate) fn print_error(e: &RpcError) -> String {
    if e.code == -32601 {
        "This runner is too old for that. Update it and try again.".to_owned()
    } else {
        e.message.clone()
    }
}

// ---------------------------------------------------------------- the sheets

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SheetKind {
    /// Edit with Agent: what should change, sent to the thread that made it.
    Ask,
    /// Edit Text: a doc's Markdown, a diagram's Mermaid, a list's or board's
    /// Markdown, by hand. Save makes a new version.
    Edit,
}

/// `AskPageSheet` and `EditDocSheet`.
pub(crate) struct PrintSheet {
    app: WeakEntity<Threadbear>,
    client: Arc<Client>,
    pub(crate) page: Page,
    pub(crate) kind: SheetKind,
    pub(crate) text: Entity<TextareaState>,
    /// Edit Text's source has come.
    pub(crate) loaded: bool,
    pub(crate) saving: bool,
    /// Why the last save didn't go, said in the sheet (a diagram that
    /// couldn't be drawn).
    pub(crate) error: Option<String>,
    _subs: Vec<Subscription>,
}

impl PrintSheet {
    fn new(app: WeakEntity<Threadbear>, client: Arc<Client>, page: Page, kind: SheetKind, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text = cx.new(|cx| {
            let state = TextareaState::new(window, cx);
            if kind == SheetKind::Ask { state.placeholder(words::ASK_PLACEHOLDER) } else { state }
        });
        let sub = cx.observe(&text, |_, _, cx| cx.notify());
        let mut sheet = PrintSheet { app, client, page, kind, text, loaded: kind == SheetKind::Ask, saving: false, error: None, _subs: vec![sub] };
        if kind == SheetKind::Edit {
            sheet.load(window, cx);
        }
        sheet
    }

    /// Its source, for the editor; an older runner may have none.
    fn load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (client, id, diagram) = (self.client.clone(), self.page.id.clone(), self.page.is_diagram());
        cx.spawn_in(window, async move |this, cx| {
            let res = call(client, "pages.get", json!({ "pageId": id })).await;
            this.update_in(cx, |this, window, cx| {
                let source = res.as_ref().ok().and_then(|v| v[if diagram { "mermaid" } else { "markdown" }].as_str().map(str::to_owned));
                match (source, res) {
                    (Some(source), _) => {
                        this.text.update(cx, |t, cx| t.set_value(source, window, cx));
                        this.loaded = true;
                    }
                    (None, Ok(_)) => this.error = Some(words::NO_SOURCE.to_owned()),
                    (None, Err(e)) => this.error = Some(print_error(&e)),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(crate) fn close(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        if let Some(app) = self.app.upgrade() {
            app.update(cx, |app, cx| {
                app.prints.sheet = None;
                cx.notify();
            });
        }
    }

    fn value(&self, cx: &App) -> String {
        self.text.read(cx).value().to_string()
    }

    pub(crate) fn can_go(&self, cx: &App) -> bool {
        match self.kind {
            SheetKind::Ask => !self.value(cx).trim().is_empty(),
            SheetKind::Edit => self.loaded && !self.saving,
        }
    }

    pub(crate) fn go(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_go(cx) {
            return;
        }
        let text = self.value(cx);
        match self.kind {
            SheetKind::Ask => {
                let id = self.page.id.clone();
                if let Some(app) = self.app.upgrade() {
                    app.update(cx, |app, cx| app.edit_print_with_agent(&id, text, cx));
                }
                self.close(window, cx);
            }
            SheetKind::Edit => {
                self.saving = true;
                self.error = None;
                cx.notify();
                let (client, id) = (self.client.clone(), self.page.id.clone());
                let (method, params) = if self.page.is_diagram() { ("pages.saveDiagram", json!({ "pageId": id, "mermaid": text })) } else { ("pages.saveDoc", json!({ "pageId": id, "markdown": text })) };
                cx.spawn_in(window, async move |this, cx| {
                    let res = call(client, method, params).await;
                    this.update_in(cx, |this, window, cx| {
                        this.saving = false;
                        match res {
                            Ok(v) => {
                                if let (Some(page), Some(app)) = (Page::decode(&v["page"]), this.app.upgrade()) {
                                    app.update(cx, |app, cx| app.replace_print(page, cx));
                                }
                                this.close(window, cx);
                            }
                            // Kept open with what was typed, and why.
                            Err(e) => this.error = Some(print_error(&e)),
                        }
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
        }
    }

    #[cfg(feature = "automation")]
    pub(crate) fn dump(&self, cx: &App) -> Value {
        json!({ "kind": if self.kind == SheetKind::Ask { "ask" } else { "edit" }, "page": self.page.id, "title": self.page.title, "text": self.value(cx), "loaded": self.loaded, "saving": self.saving, "error": self.error })
    }
}

impl Render for PrintSheet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let secondary = style::secondary_text(cx);
        let can_go = self.can_go(cx);
        let edit = self.kind == SheetKind::Edit;
        let mono = cx.theme().mono_font_family.clone();
        let height = if edit { 360. } else { 84. };
        let field = div()
            .id("print-sheet-text")
            .h(px(height))
            .rounded(px(6.))
            .border(gpui_kit::px(0.5))
            .border_color(style::hairline(cx))
            .bg(style::primary_opacity(cx, 0.035))
            .tip(match (self.kind, self.page.is_diagram()) {
                (SheetKind::Ask, _) => "What should change on the print",
                (SheetKind::Edit, true) => "The diagram's Mermaid source. Save makes a new version; the old one is kept",
                (SheetKind::Edit, false) => "The text. Save makes a new version; the old one is kept",
            })
            .when(edit, |s| s.font_family(mono).text_size(px(style::size::LABEL)))
            .child(if self.loaded || !edit {
                Textarea::new(&self.text).h(px(height)).appearance(false).into_any_element()
            } else {
                div().size_full().flex().items_center().justify_center().child(Spinner::new().color(secondary).with_size(px(14.))).into_any_element()
            });
        let header = if edit {
            h_flex()
                .gap(px(12.))
                .items_baseline()
                .child(div().flex_none().text_size(px(style::size::CARD_TITLE)).font_semibold().child(self.page.title.clone()))
                .child(div().flex_1().min_w_0().text_right().text_size(px(style::size::DETAIL)).text_color(secondary).child(if self.page.is_diagram() { words::DIAGRAM_HINT } else { words::DOC_HINT }))
                .into_any_element()
        } else {
            v_flex()
                .gap(px(4.))
                .child(div().text_size(px(style::size::CARD_TITLE)).font_semibold().child(format!("Change \u{201C}{}\u{201D}", self.page.title)))
                .child(div().text_size(px(style::size::DETAIL)).text_color(secondary).child(words::ASK_ABOUT))
                .into_any_element()
        };
        let (go_label, go_tip) = if edit { ("Save", "Save your edit as a new version of the print") } else { ("Send", "Send this to the thread that made the print, and open that thread") };
        v_flex()
            .gap(px(10.))
            .child(header)
            .child(field)
            .when_some(self.error.clone(), |s, e| s.child(div().text_size(px(style::size::LABEL)).text_color(style::failed(cx)).child(e)))
            .child(
                h_flex()
                    .gap(px(8.))
                    .items_center()
                    .when(edit, |s| s.child(div().text_size(px(style::size::DETAIL)).text_color(secondary).child(format!("Saves as version {}.", self.page.version + 1))))
                    .child(div().flex_1())
                    .child(Button::new("print-sheet-cancel").label("Cancel").small().outline().tooltip(if edit { "Close without saving" } else { "Close without asking for a change" }).on_click(cx.listener(|this, _, window, cx| this.close(window, cx))))
                    .child(Button::new("print-sheet-go").label(go_label).small().primary().disabled(!can_go).loading(self.saving).tooltip(go_tip).on_click(cx.listener(|this, _, window, cx| this.go(window, cx)))),
            )
    }
}

// ---------------------------------------------------------------- scripts

/// The dump's `prints`: whether they're on, each one as its row says, and
/// the sheet while it's open.
#[cfg(feature = "automation")]
pub(crate) fn dump(app: &Threadbear, cx: &App) -> Value {
    if !app.prints_on() {
        return Value::Null;
    }
    let titles: HashMap<String, String> = app.projects.list.iter().map(|p| (p.id.clone(), p.title.clone())).collect();
    let now = now_ms();
    json!({
        "list": core::rows::all(&app.prints.list).iter().map(|p| json!({ "id": p.id, "title": p.title, "detail": p.row_detail(now), "projects": core::rows::projects(p, &titles), "version": p.version, "busy": app.print_busy(&p.id) })).collect::<Vec<_>>(),
        "deleted": app.prints.deleted.iter().map(|p| p.title.clone()).collect::<Vec<_>>(),
        // What the open project's page draws in its Prints section.
        "projectPage": app.projects.open.as_ref().map(|id| core::rows::in_project(id, &app.prints.list).iter().map(|p| format!("{} {}", p.title, p.row_detail(now))).collect::<Vec<_>>()),
        "sheet": app.prints.sheet.as_ref().map_or(Value::Null, |s| s.read(cx).dump(cx)),
        // Picked with ⌘- or Shift-click, by title, and the Delete asking.
        "picked": app.picked_prints().iter().filter_map(|id| app.print(id)).map(|p| p.title.clone()).collect::<Vec<_>>(),
        "askingDelete": app.prints.asking_delete.as_ref().map(|ids| ids.iter().filter_map(|id| app.print(id)).map(|p| p.title.clone()).collect::<Vec<_>>()),
    })
}

/// `print`: `{ id, do: "menu" | "open" | "here" | "ask" | "edit" | "type" | "save" | "close" | "pdf" | "link" | "pick" | "pickedMenu" | "deletePicked" | "confirm" | "clear", text?, x?, y?, path?, url?, with? }`:
/// a Print's menu opened (at x, y), its window or side pane, its sheets,
/// and what's typed in a sheet and saved.
#[cfg(feature = "automation")]
pub(crate) fn automation(app: &Entity<Threadbear>, cmd: &Value, window: &mut Window, cx: &mut App) -> Result<Value, String> {
    let what = cmd["do"].as_str().unwrap_or("menu");
    // A click on a todo list's box or a board's arrow in the side pane, as
    // the page there sends it (web_panels.rs): the stand-in for a real click.
    if what == "link" {
        let url = cmd["url"].as_str().ok_or("give the link's url")?.to_owned();
        let side = app.read(cx).side.clone();
        let thread = app.read(cx).pane_thread().ok_or("no thread open")?;
        let file = side.read(cx).previews.get(&thread.id).and_then(|p| p.file_path().map(str::to_owned)).ok_or("nothing open beside the thread")?;
        app.update(cx, |a, cx| a.handle_print_link(&url, &file, cx));
        return Ok(json!(format!("ok link {file}")));
    }
    if matches!(what, "type" | "save" | "close") {
        let sheet = app.read(cx).prints.sheet.clone().ok_or("no Print sheet is open")?;
        match what {
            "type" => {
                let text = cmd["text"].as_str().unwrap_or("").to_owned();
                let field = sheet.read(cx).text.clone();
                field.update(cx, |f, cx| f.set_value(text, window, cx));
            }
            "save" => sheet.update(cx, |s, cx| s.go(window, cx)),
            _ => sheet.update(cx, |s, cx| s.close(window, cx)),
        }
        return Ok(json!("ok"));
    }
    // Several at once (print_select.rs): `pickedMenu` opens the picked
    // ones' menu, `deletePicked` asks, `confirm` answers Delete, `clear` is Esc.
    if matches!(what, "pickedMenu" | "deletePicked" | "confirm" | "clear" | "projectPicked") {
        app.update(cx, |a, cx| match what {
            // The picked ones into the project titled `project` (out with `add: false`).
            "projectPicked" => {
                let ids = a.picked_prints();
                let title = cmd["project"].as_str().unwrap_or_default();
                if let Some(project) = a.projects.list.iter().find(|p| p.title == title || p.id == title).map(|p| p.id.clone()) {
                    a.set_prints_project(ids, project, cmd["add"].as_bool().unwrap_or(true), cx);
                }
            }
            "pickedMenu" => {
                let ids = a.picked_prints();
                let rows = a.picked_menu_rows(&ids);
                a.open_menu(None, rows, point(gpui_kit::px(400.), gpui_kit::px(300.)), false, window, cx);
            }
            "deletePicked" => {
                let ids = a.picked_prints();
                a.ask_delete_prints(ids, window, cx);
            }
            "confirm" => {
                if let Some(ids) = a.prints.asking_delete.clone() {
                    a.delete_prints(ids, cx);
                }
            }
            _ => {
                a.clear_picked_prints(cx);
            }
        });
        if what == "confirm" {
            window.close_dialog(cx);
        }
        return Ok(json!(format!("ok {what}")));
    }
    let key = cmd["id"].as_str().ok_or("give a Print's id or title")?;
    let id = {
        let a = app.read(cx);
        a.prints.list.iter().chain(a.prints.deleted.iter()).find(|p| p.id == key || p.title == key).map(|p| p.id.clone()).ok_or_else(|| format!("no Print {key}"))?
    };
    app.update(cx, |a, cx| match what {
        "menu" => {
            let at = point(gpui_kit::px(cmd["x"].as_f64().unwrap_or(400.) as f32), gpui_kit::px(cmd["y"].as_f64().unwrap_or(300.) as f32));
            a.open_print_menu(&id, cmd["here"].as_bool().unwrap_or(false), at, window, cx);
        }
        "open" => a.open_print_window(&id, None, cx),
        "here" => a.open_print_here(&id, window, cx),
        "ask" => a.open_print_sheet(&id, SheetKind::Ask, window, cx),
        "edit" => a.open_print_sheet(&id, SheetKind::Edit, window, cx),
        // Download as PDF without the Save dialog, which a script can't
        // answer: to `path`.
        "pdf" => {
            let dest = std::path::PathBuf::from(cmd["path"].as_str().unwrap_or("/tmp/print.pdf"));
            let title = a.print(&id).map(|p| p.title.clone()).unwrap_or_default();
            a.print_call(&id, None, "pages.get", json!({ "pageId": id }), cx, move |this, v, cx| {
                if let Some(file) = v["file"].as_str() {
                    this.save_print_pdf(file.to_owned(), dest, title, cx);
                }
            });
        }
        "deleted" => {
            for p in a.prints.list.iter().chain(a.prints.deleted.iter()).filter(|p| p.id == id) {
                for r in &p.projects {
                    a.prints.show_deleted.insert(r.id.clone());
                }
            }
            cx.notify();
        }
        // A click on its row in the rail's list, with `with` held: "cmd",
        // "shift" or nothing (which opens it, and lets the picked go).
        "pick" => {
            let pick = match cmd["with"].as_str() {
                Some("cmd") => crate::print_select::Pick::Toggle,
                Some("shift") => crate::print_select::Pick::Range,
                _ => crate::print_select::Pick::Plain,
            };
            let order: Vec<String> = core::rows::all(&a.prints.list).iter().map(|p| p.id.clone()).collect();
            if !a.print_row_clicked(&id, &order, pick, cx) {
                a.open_print_window(&id, None, cx);
            }
        }
        other => a.show_error(format!("no Print action {other}"), cx),
    });
    Ok(json!(format!("ok {what} {id}")))
}
