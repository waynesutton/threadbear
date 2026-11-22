// Visible, scoped selection for the three sidebar lists. Controls share their
// layout; each list keeps its own query and archive view. Deferred callbacks
// release SidePane before an action opens another panel or dialog.
use crate::{
    app::Threadbear,
    highlights::SidePane,
    style::{self, px},
};
use gpui_kit::base::TestTextExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::Input;
use gpui_kit::component::{
    h_flex, v_flex, ActiveTheme as _, Disableable as _, Sizable as _, WindowExt as _,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Prints,
    Bears,
    Tables,
}
impl Kind {
    pub(crate) const ALL: [Self; 3] = [Self::Prints, Self::Bears, Self::Tables];
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Prints => 0,
            Self::Bears => 1,
            Self::Tables => 2,
        }
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Prints => "Prints",
            Self::Bears => "Bears",
            Self::Tables => "Tables",
        }
    }
    fn accepts(self, key: &str) -> bool {
        match self {
            Self::Prints => true,
            Self::Bears => key.starts_with("bear:"),
            Self::Tables => key.starts_with("table:"),
        }
    }
}
#[derive(Default)]
pub(crate) struct ListState {
    pub selecting: bool,
    pub archived: bool,
    pub query: String,
}
#[derive(Clone, Copy)]
enum Action {
    Select,
    All,
    Archive,
    Delete,
    View,
    New,
}

impl Threadbear {
    pub(crate) fn sidebar_list(&self, kind: Kind) -> &ListState {
        match kind {
            Kind::Prints => &self.prints.sidebar,
            Kind::Bears => &self.bears_panel.bears_view,
            Kind::Tables => &self.bears_panel.tables_view,
        }
    }
    fn sidebar_list_mut(&mut self, kind: Kind) -> &mut ListState {
        match kind {
            Kind::Prints => &mut self.prints.sidebar,
            Kind::Bears => &mut self.bears_panel.bears_view,
            Kind::Tables => &mut self.bears_panel.tables_view,
        }
    }
    pub(crate) fn sidebar_search(&mut self, kind: Kind, query: String, cx: &mut Context<Self>) {
        self.sidebar_list_mut(kind).query = query;
        self.sidebar_clear(kind, cx);
    }
    pub(crate) fn sidebar_clear(&mut self, kind: Kind, cx: &mut Context<Self>) {
        match kind {
            Kind::Prints => {
                self.prints.picked.clear();
                self.prints.anchor = None;
            }
            _ => self.bears_panel.selected_mut(kind).clear(),
        }
        cx.notify();
    }
    pub(crate) fn visible_prints(&self) -> Vec<threadbear_core::prints::Page> {
        let state = &self.prints.sidebar;
        let archived = self
            .settings
            .saved
            .archived_print_ids
            .as_deref()
            .unwrap_or_default();
        let query = state.query.trim().to_lowercase();
        threadbear_core::prints::rows::all(&self.prints.list)
            .into_iter()
            .filter(|p| {
                archived.contains(&p.id) == state.archived
                    && p.title.to_lowercase().contains(&query)
            })
            .collect()
    }
    pub(crate) fn visible_bears(&self, kind: Kind) -> Vec<crate::bears_panel::Row> {
        let state = self.sidebar_list(kind);
        let query = state.query.trim().to_lowercase();
        self.bears_panel_rows()
            .into_iter()
            .filter(|r| {
                kind.accepts(&r.key)
                    && (r.section == threadbear_core::bears_panel::ARCHIVED) == state.archived
                    && r.title.to_lowercase().contains(&query)
            })
            .collect()
    }
    pub(crate) fn sidebar_ids(&self, kind: Kind) -> Vec<String> {
        match kind {
            Kind::Prints => self.visible_prints().iter().map(|p| p.id.clone()).collect(),
            _ => self
                .visible_bears(kind)
                .iter()
                .map(|r| r.key.clone())
                .collect(),
        }
    }
    pub(crate) fn sidebar_selected(&self, kind: Kind) -> Vec<String> {
        let visible = self.sidebar_ids(kind);
        let picked = match kind {
            Kind::Prints => &self.prints.picked,
            _ => &self.bears_panel.selected(kind).keys,
        };
        picked
            .iter()
            .filter(|id| visible.contains(id))
            .cloned()
            .collect()
    }
    pub(crate) fn sidebar_all(&mut self, kind: Kind, cx: &mut Context<Self>) {
        let ids = self.sidebar_ids(kind);
        self.sidebar_list_mut(kind).selecting = true;
        match kind {
            Kind::Prints => {
                self.prints.anchor = ids.first().cloned();
                self.prints.picked = ids;
            }
            _ => {
                let selected = self.bears_panel.selected_mut(kind);
                selected.anchor = ids.first().cloned();
                selected.keys = ids;
            }
        }
        cx.notify();
    }
    fn sidebar_action(
        &mut self,
        kind: Kind,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if kind != Kind::Prints && self.bears_panel.archiving {
            return;
        }
        match action {
            Action::Select => {
                let selecting =
                    !(self.sidebar_list(kind).selecting || !self.sidebar_selected(kind).is_empty());
                self.sidebar_clear(kind, cx);
                self.sidebar_list_mut(kind).selecting = selecting;
            }
            Action::All => self.sidebar_all(kind, cx),
            // Bears and tables are made here, in their own lists.
            Action::New => {
                let action = if kind == Kind::Tables {
                    crate::bears_panel::BearsAction::NewTable
                } else {
                    crate::bears_panel::BearsAction::NewBear
                };
                if kind != Kind::Prints {
                    self.run_bears_action(action, window, cx);
                }
            }
            Action::View => {
                self.sidebar_list_mut(kind).archived = !self.sidebar_list(kind).archived;
                self.sidebar_clear(kind, cx);
            }
            Action::Delete => {
                let ids = self.sidebar_selected(kind);
                match kind {
                    Kind::Prints => self.ask_delete_prints(ids, window, cx),
                    _ => self.open_bears_delete(ids, window, cx),
                }
            }
            Action::Archive => {
                let ids = self.sidebar_selected(kind);
                if ids.is_empty() {
                    return;
                }
                let archived = !self.sidebar_list(kind).archived;
                if kind == Kind::Prints {
                    let saved = self
                        .settings
                        .saved
                        .archived_print_ids
                        .get_or_insert_with(Vec::new);
                    for id in &ids {
                        saved.retain(|x| x != id);
                        if archived {
                            saved.push(id.clone());
                        }
                    }
                    self.sidebar_clear(kind, cx);
                    self.save_settings_soon(cx);
                    self.flash_done(
                        if archived {
                            "Prints archived on this Mac"
                        } else {
                            "Prints restored"
                        },
                        cx,
                    );
                } else {
                    self.ask_sidebar_archive(kind, ids, archived, window, cx);
                }
            }
        }
        cx.notify();
    }
    fn ask_sidebar_archive(
        &mut self,
        kind: Kind,
        ids: Vec<String>,
        archived: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let verb = if archived { "Archive" } else { "Unarchive" };
        let names: Vec<String> = self
            .bears_panel_rows()
            .iter()
            .filter(|r| ids.contains(&r.key))
            .map(|r| r.title.clone())
            .collect();
        let detail = match (kind, archived) {
            (Kind::Bears, true) => "Their routines will pause, webhooks will be revoked, and they will leave their tables. Folders and threads stay. Archiving the Head Bear leaves no active head.",
            (Kind::Bears, false) => "Routines stay paused. Webhooks and table seats must be added again.",
            _ => "Folders, threads and seated bears stay. You can change this again in the Archived view.",
        };
        let title = format!("{verb} {} {}?", ids.len(), kind.name().to_lowercase());
        let description = format!("{}\n\n{detail}", names.join(", "));
        let app = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (app, ids) = (app.clone(), ids.clone());
            alert
                .title(title.clone())
                .description(description.clone())
                .button_props(DialogButtonProps::default().ok_text(verb).show_cancel(true))
                .on_ok(move |_, _, cx| {
                    app.update(cx, |a, cx| {
                        a.archive_sidebar_rows(kind, ids.clone(), archived, cx)
                    })
                    .ok();
                    true
                })
        });
    }
    fn archive_sidebar_rows(
        &mut self,
        kind: Kind,
        ids: Vec<String>,
        archived: bool,
        cx: &mut Context<Self>,
    ) {
        if self.bears_panel.archiving {
            return;
        }
        self.bears_panel.archiving = true;
        self.begin_note("sidebarArchive", "Updating archive…", cx);
        let client = self.client.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let mut failed = vec![];
            for key in &ids {
                let (method, params) = if let Some(id) = key.strip_prefix("bear:") {
                    ("bears.archive", json!({"bear": id, "archived": archived}))
                } else if let Some(id) = key.strip_prefix("table:") {
                    (
                        "projects.update",
                        json!({"projectId": id, "archived": archived}),
                    )
                } else {
                    continue;
                };
                match crate::call(client.clone(), method, params).await {
                    Ok(_) => {
                        this.update(cx, |a, cx| {
                            if let Some(id) = key.strip_prefix("bear:") {
                                if let Some(b) = a
                                    .bears_panel
                                    .bears
                                    .as_mut()
                                    .and_then(|bs| bs.iter_mut().find(|b| b.id == id))
                                {
                                    b.archived = archived;
                                }
                            } else if let Some(id) = key.strip_prefix("table:") {
                                if let Some(p) = a.projects.list.iter_mut().find(|p| p.id == id) {
                                    p.archived = archived;
                                }
                            }
                            cx.notify();
                        })
                        .ok();
                    }
                    Err(e) => failed.push((key.clone(), e.message)),
                }
            }
            this.update(cx, |a, cx| {
                a.bears_panel.archiving = false;
                a.end_note("sidebarArchive", cx);
                a.sidebar_clear(kind, cx);
                a.bears_panel.selected_mut(kind).keys =
                    failed.iter().map(|(id, _)| id.clone()).collect();
                if let Some((_, message)) = failed.first() {
                    a.show_error(
                        format!("{} item(s) could not be updated: {message}", failed.len()),
                        cx,
                    );
                }
                a.refresh_bears_panel(true, cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl SidePane {
    pub(crate) fn sidebar_controls(&mut self, kind: Kind, cx: &mut Context<Self>) -> AnyElement {
        let Some(app) = self.app.upgrade() else {
            return div().into_any_element();
        };
        let a = app.read(cx);
        let state = a.sidebar_list(kind);
        let (selecting, archived, count, total) = (
            state.selecting,
            state.archived,
            a.sidebar_selected(kind).len(),
            a.sidebar_ids(kind).len(),
        );
        let busy = kind != Kind::Prints && a.bears_panel.archiving;
        let active = selecting || count > 0;
        let focus = self.list_focus[kind.index()].clone();
        let button = |label: &'static str, action: Action, disabled: bool, tip: String| {
            let (app, focus) = (app.downgrade(), focus.clone());
            Button::new(SharedString::from(format!("{}-{label}", kind.name())))
                .label(label)
                .accessibility_label(format!("{label} {}", kind.name()))
                .ghost()
                .xsmall()
                .disabled(disabled)
                .tooltip(tip)
                .on_click(move |_, window, cx| {
                    window.focus(&focus, cx);
                    crate::web_panels::keyboard_to_app(window);
                    let (app, handle) = (app.clone(), window.window_handle());
                    cx.defer(move |cx| {
                        handle
                            .update(cx, |_, window, cx| {
                                app.update(cx, |a, cx| a.sidebar_action(kind, action, window, cx))
                                    .ok();
                            })
                            .ok();
                    });
                })
        };
        v_flex()
            .flex_none()
            .px(px(10.))
            .py(px(6.))
            .gap(px(6.))
            .child(
                Input::new(&self.list_search[kind.index()])
                    .small()
                    .cleanable(true),
            )
            .child(
                h_flex()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(style::size::SMALL))
                            .flex_1()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{} {}",
                                total,
                                if archived {
                                    "archived"
                                } else if total == 1 {
                                    "item"
                                } else {
                                    "items"
                                }
                            ))
                            .test_text(format!(
                                "{} {}",
                                total,
                                if archived {
                                    "archived"
                                } else if total == 1 {
                                    "item"
                                } else {
                                    "items"
                                }
                            )),
                    )
                    .when(kind != Kind::Prints && !archived && !active, |row| {
                        row.child(button(
                            "New",
                            Action::New,
                            busy,
                            if kind == Kind::Tables {
                                "Make a table: pick its bears, its project and a photo".into()
                            } else {
                                "Make a bear: its name, what it does and a photo".into()
                            },
                        ))
                    })
                    .child(button(
                        if archived { "Active" } else { "Archived" },
                        Action::View,
                        busy,
                        format!(
                            "Show {} {}",
                            if archived { "active" } else { "archived" },
                            kind.name()
                        ),
                    ))
                    .child(button(
                        if active { "Done" } else { "Select" },
                        Action::Select,
                        busy,
                        format!(
                            "{} {} selection",
                            if active { "Finish" } else { "Start" },
                            kind.name()
                        ),
                    )),
            )
            .when(active, |view| {
                view.child(
                    h_flex()
                        .gap(px(3.))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(style::size::SMALL))
                                .child(format!("{count} selected"))
                                .test_text(format!("{count} selected")),
                        )
                        .child(button(
                            "All",
                            Action::All,
                            busy || total == 0,
                            format!("Select all visible {}", kind.name()),
                        ))
                        .child(button(
                            if archived { "Restore" } else { "Archive" },
                            Action::Archive,
                            busy || count == 0,
                            if kind == Kind::Prints {
                                "Archive on this Mac. Files and links stay intact".into()
                            } else {
                                "Change the archive status of the selected items".into()
                            },
                        ))
                        .child(button(
                            "Delete",
                            Action::Delete,
                            busy || count == 0,
                            "Delete the selected items, after confirmation".into(),
                        )),
                )
            })
            .into_any_element()
    }
    pub(crate) fn sidebar_keys(
        &mut self,
        kind: Kind,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Cmd+A inside the search field continues to select its text.
        if self.list_search[kind.index()]
            .focus_handle(cx)
            .contains_focused(window, cx)
        {
            return;
        }
        if let Some(app) = self.app.upgrade() {
            if ev.keystroke.key == "a" && ev.keystroke.modifiers.platform {
                app.update(cx, |a, cx| a.sidebar_all(kind, cx));
                cx.stop_propagation();
            } else if ev.keystroke.key == "escape" {
                app.update(cx, |a, cx| {
                    a.sidebar_list_mut(kind).selecting = false;
                    a.sidebar_clear(kind, cx);
                });
                cx.stop_propagation();
            }
        }
    }
}

#[cfg(feature = "automation")]
pub(crate) fn dump(app: &Threadbear) -> serde_json::Value {
    serde_json::Value::Object(Kind::ALL.into_iter().map(|kind| {
        let state = app.sidebar_list(kind);
        (kind.name().into(), json!({"query": state.query, "selecting": state.selecting,
            "archived": state.archived, "visible": app.sidebar_ids(kind), "selected": app.sidebar_selected(kind),
            "busy": kind != Kind::Prints && app.bears_panel.archiving}))
    }).collect())
}
