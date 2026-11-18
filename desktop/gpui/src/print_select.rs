// Several Prints at once (Wayne, 17 Nov): in the rail's Prints, Highlights'
// Prints and a project page's Prints, ⌘-click picks a Print or lets it go,
// Shift-click picks every one from the last picked to it, and Esc lets them
// all go. A plain click opens one as it always did, and lets the rest go.
// What's picked is drawn picked, and a bar over the list says how many with
// Projects and Delete for them all; a right click on a picked one has the
// same in its menu. Delete asks once, naming them, takes the rows away at
// once and puts back any the runner wouldn't delete, saying why.
use crate::app::Threadbear;
use crate::prints::PrintAction;
use crate::sidebar::{MenuAction, MenuRow};
use crate::style::{self, px};
use crate::tips::Tip as _;
use crate::{call, icons};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::{h_flex, ActiveTheme as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::json;
use threadbear_core::prints::Page;

/// What a click on a Print's row does to what's picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pick {
    /// ⌘: this one in or out.
    Toggle,
    /// Shift: every one from the last picked to this.
    Range,
    /// No key held: opens it, and nothing stays picked.
    Plain,
}

impl Pick {
    pub(crate) fn from(modifiers: &Modifiers) -> Pick {
        let command = if cfg!(target_os = "macos") { modifiers.platform } else { modifiers.control };
        if command {
            Pick::Toggle
        } else if modifiers.shift {
            Pick::Range
        } else {
            Pick::Plain
        }
    }
}

/// What's picked after a click on `id` in a list drawn in `order`: `picked`
/// and `anchor` as they were, the new ones out. A pure function, so it's
/// tested without a window.
pub(crate) fn picked_after(picked: &[String], anchor: Option<&str>, id: &str, order: &[String], pick: Pick) -> (Vec<String>, Option<String>) {
    match pick {
        Pick::Plain => (vec![], None),
        Pick::Toggle => {
            let mut now: Vec<String> = picked.to_vec();
            if let Some(at) = now.iter().position(|p| p == id) {
                now.remove(at);
            } else {
                now.push(id.to_owned());
            }
            (now, Some(id.to_owned()))
        }
        Pick::Range => {
            let from = anchor.and_then(|a| order.iter().position(|o| o == a));
            let Some(to) = order.iter().position(|o| o == id) else { return (picked.to_vec(), anchor.map(str::to_owned)) };
            let Some(from) = from else { return (vec![id.to_owned()], Some(id.to_owned())) };
            let (a, b) = if from <= to { (from, to) } else { (to, from) };
            (order[a..=b].to_vec(), anchor.map(str::to_owned))
        }
    }
}

/// "3 prints", "1 print".
pub(crate) fn count(n: usize) -> String {
    if n == 1 { "1 print".to_owned() } else { format!("{n} prints") }
}

/// The titles a confirm names: all of them up to five, then how many more.
pub(crate) fn naming(titles: &[String]) -> String {
    let quoted: Vec<String> = titles.iter().take(5).map(|t| format!("\u{201C}{t}\u{201D}")).collect();
    let mut out = quoted.join(", ");
    if titles.len() > 5 {
        out.push_str(&format!(" and {} more", titles.len() - 5));
    }
    out
}

impl Threadbear {
    /// The Prints picked, those still listed, in the order they were picked.
    pub(crate) fn picked_prints(&self) -> Vec<String> {
        self.prints.picked.iter().filter(|id| self.prints.list.iter().any(|p| &p.id == *id)).cloned().collect()
    }

    pub(crate) fn print_picked(&self, id: &str) -> bool {
        self.prints.picked.iter().any(|p| p == id)
    }

    /// A click on a Print's row in a list drawn in `order`. True when it
    /// changed what's picked (⌘ or Shift), so the row doesn't open it.
    pub(crate) fn print_row_clicked(&mut self, id: &str, order: &[String], pick: Pick, cx: &mut Context<Self>) -> bool {
        if pick == Pick::Plain && self.prints.picked.is_empty() {
            return false;
        }
        let (now, anchor) = picked_after(&self.prints.picked, self.prints.anchor.as_deref(), id, order, pick);
        self.prints.picked = now;
        self.prints.anchor = anchor;
        cx.notify();
        pick != Pick::Plain
    }

    /// Esc, or the bar's Clear: nothing picked.
    pub(crate) fn clear_picked_prints(&mut self, cx: &mut Context<Self>) -> bool {
        if self.prints.picked.is_empty() {
            return false;
        }
        self.prints.picked.clear();
        self.prints.anchor = None;
        cx.notify();
        true
    }

    /// A right click on a Print's row: the picked ones' menu when it's one
    /// of two or more picked, else its own (and nothing stays picked).
    pub(crate) fn print_row_menu(&mut self, id: &str, here: bool, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let picked = self.picked_prints();
        if picked.len() > 1 && picked.iter().any(|p| p == id) {
            let rows = self.picked_menu_rows(&picked);
            self.open_menu(None, rows, position, false, window, cx);
            return;
        }
        self.clear_picked_prints(cx);
        self.open_print_menu(id, here, position, window, cx);
    }

    /// What can be done to several at once: Add to or Remove from each
    /// project (a tick when every one is in it), and Delete.
    pub(crate) fn picked_menu_rows(&self, ids: &[String]) -> Vec<MenuRow> {
        let mut rows = vec![];
        let add = self.picked_project_rows(ids);
        if !add.is_empty() {
            rows.push(MenuRow::submenu(format!("Projects for {}", count(ids.len())), "square.stack", add));
            rows.push(MenuRow::separator());
        }
        rows.push(MenuRow::item(format!("Delete {}\u{2026}", count(ids.len())), "trash", MenuAction::Print(PrintAction::DeleteMany(ids.to_vec()))));
        rows
    }

    /// A line for each project, ticked when every one is in it: a click
    /// takes them all out, else puts them all in.
    pub(crate) fn picked_project_rows(&self, ids: &[String]) -> Vec<MenuRow> {
        let pages: Vec<&Page> = ids.iter().filter_map(|id| self.print(id)).collect();
        self
            .projects
            .list
            .iter()
            .filter(|p| !p.archived)
            .map(|p| {
                let all_in = !pages.is_empty() && pages.iter().all(|page| page.is_in(&p.id));
                MenuRow::check(p.title.clone(), "square.stack", all_in, MenuAction::Print(PrintAction::ProjectMany(ids.to_vec(), p.id.clone(), !all_in)))
            })
            .collect()
    }

    /// Asks once, naming them; Delete takes them all away.
    pub(crate) fn ask_delete_prints(&mut self, ids: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.sidebar.menu = None;
        let titles: Vec<String> = ids.iter().filter_map(|id| self.print(id)).map(|p| p.title.clone()).collect();
        if titles.is_empty() {
            return;
        }
        let me = cx.entity().downgrade();
        self.prints.asking_delete = Some(ids.clone());
        cx.notify();
        let title = format!("Delete {}?", count(titles.len()));
        let detail = format!("{}. Deleted prints are kept for 7 days.", naming(&titles));
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (ok, closed, ids) = (me.clone(), me.clone(), ids.clone());
            alert
                .title(title.clone())
                .description(detail.clone())
                // Delete is the filled (primary) button, with the ⏎ Allow shows on an
                // approval: what Return does.
                .button_props(DialogButtonProps::default().ok_text("Delete \u{23CE}").show_cancel(true))
                .on_ok(move |_, _, cx| {
                    ok.update(cx, |this, cx| this.delete_prints(ids.clone(), cx)).ok();
                    true
                })
                .on_close(move |_, _, cx| {
                    closed
                        .update(cx, |this, cx| {
                            this.prints.asking_delete = None;
                            cx.notify();
                        })
                        .ok();
                })
        });
        // Return deletes and Esc cancels (Wayne, 18 Nov): the dialog takes
        // GPUI's focus as it opens, but the Print's page beside the list
        // kept AppKit's keyboard, so neither key reached it. Again on the
        // next frame, as the menu that asked closes after this.
        crate::web_panels::keyboard_to_app(window);
        window.defer(cx, |window, _| crate::web_panels::keyboard_to_app(window));
    }

    /// The rows go at once; each is deleted on the runner, and any it
    /// wouldn't delete come back where they were, with why.
    pub(crate) fn delete_prints(&mut self, ids: Vec<String>, cx: &mut Context<Self>) {
        self.prints.asking_delete = None;
        let ids: Vec<String> = ids.into_iter().filter(|id| self.prints.list.iter().any(|p| &p.id == id) && !self.print_busy(id)).collect();
        if ids.is_empty() {
            return;
        }
        let before = self.prints.list.clone();
        self.prints.list.retain(|p| !ids.contains(&p.id));
        self.prints.picked.retain(|p| !ids.contains(p));
        self.prints.anchor = None;
        for id in &ids {
            self.prints.busy.insert(id.clone());
        }
        let key = format!("prints:delete:{}", ids.join(","));
        self.begin_note(&key, &format!("Deleting {}", count(ids.len())), cx);
        crate::print_cards::changed(cx);
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let mut gone = vec![];
            let mut failed: Vec<(String, String)> = vec![];
            for id in &ids {
                match call(client.clone(), "pages.delete", json!({ "pageId": id })).await {
                    Ok(v) => gone.extend(Page::decode(&v["page"])),
                    Err(e) => failed.push((id.clone(), crate::prints::print_error(&e))),
                }
            }
            this.update(cx, |this, cx| {
                for id in &ids {
                    this.prints.busy.remove(id);
                }
                this.end_note(&key, cx);
                for page in gone {
                    this.prints.deleted.retain(|p| p.id != page.id);
                    this.prints.deleted.insert(0, page);
                }
                if !failed.is_empty() {
                    // Back where they were, among what's listed now.
                    let back: Vec<Page> = before.iter().filter(|p| failed.iter().any(|(id, _)| id == &p.id)).cloned().collect();
                    let mut list = this.prints.list.clone();
                    for page in back {
                        let at = before.iter().position(|p| p.id == page.id).unwrap_or(0).min(list.len());
                        list.insert(at, page);
                    }
                    this.receive_prints(list, cx);
                    let why = &failed[0].1;
                    let message = if ids.len() == 1 { why.clone() } else { format!("Couldn't delete {} of {}: {why}", failed.len(), count(ids.len())) };
                    this.show_error(message, cx);
                }
                crate::print_cards::changed(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Each into the project (`add`) or out of it; those already so are
    /// left alone. Their rows show a spinner until the runner answers.
    pub(crate) fn set_prints_project(&mut self, ids: Vec<String>, project: String, add: bool, cx: &mut Context<Self>) {
        let ids: Vec<String> = ids.into_iter().filter(|id| self.print(id).is_some_and(|p| p.is_in(&project) != add) && !self.print_busy(id)).collect();
        if ids.is_empty() {
            return;
        }
        for id in &ids {
            self.prints.busy.insert(id.clone());
        }
        let name = self.projects.list.iter().find(|p| p.id == project).map(|p| p.title.clone()).unwrap_or_default();
        let key = format!("prints:project:{project}:{}", ids.join(","));
        let note = if add { format!("Adding {} to {name}", count(ids.len())) } else { format!("Taking {} out of {name}", count(ids.len())) };
        self.begin_note(&key, &note, cx);
        crate::print_cards::changed(cx);
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let mut changed = vec![];
            let mut failed: Vec<String> = vec![];
            for id in &ids {
                let params = if add { json!({ "pageId": id, "add": [project] }) } else { json!({ "pageId": id, "remove": [project] }) };
                match call(client.clone(), "pages.setProjects", params).await {
                    Ok(v) => changed.extend(Page::decode(&v["page"])),
                    Err(e) => failed.push(crate::prints::print_error(&e)),
                }
            }
            this.update(cx, |this, cx| {
                for id in &ids {
                    this.prints.busy.remove(id);
                }
                this.end_note(&key, cx);
                if !changed.is_empty() {
                    let mut list = this.prints.list.clone();
                    for page in changed {
                        if let Some(slot) = list.iter_mut().find(|p| p.id == page.id) {
                            *slot = page;
                        }
                    }
                    this.receive_prints(list, cx);
                }
                if let Some(why) = failed.first() {
                    let message = if ids.len() == 1 { why.clone() } else { format!("Couldn't change {} of {}: {why}", failed.len(), count(ids.len())) };
                    this.show_error(message, cx);
                }
                crate::print_cards::changed(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

/// The bar over a list while any are picked: how many,
/// Projects, Delete and Clear. Its clicks go to the app, so the side pane
/// and a project's page draw the same bar. `app` is the app.
pub(crate) fn picked_bar(app: WeakEntity<Threadbear>, picked: usize, cx: &App) -> Option<AnyElement> {
    if picked == 0 {
        return None;
    }
    let secondary = style::secondary_text(cx);
    let hover = style::primary_opacity(cx, 0.08);
    let button = |id: &'static str, symbol: &'static str, label: Option<String>, tip: String| {
        div()
            .id(id)
            .role(Role::Button)
            .test_support()
            .aria_label(SharedString::from(label.clone().unwrap_or_else(|| tip.clone())))
            .h(px(24.))
            .px(px(if label.is_some() { 8. } else { 5. }))
            .gap(px(5.))
            .flex()
            .items_center()
            .rounded(px(5.))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .tip(tip)
            .child(icons::symbol(symbol).with_size(px(12.)))
            .when_some(label, |d, l| d.child(div().text_size(px(style::size::SMALL)).child(l)))
    };
    let (projects_app, delete_app, clear_app) = (app.clone(), app.clone(), app);
    let projects = button("prints-picked-projects", "square.stack", Some("Projects".into()), format!("Add the {} to a project, or take them out of one", count(picked))).on_click(move |ev: &ClickEvent, window, cx| {
        let at = ev.position();
        projects_app
            .update(cx, |this, cx| {
                let ids = this.picked_prints();
                let rows = this.picked_project_rows(&ids);
                if rows.is_empty() {
                    this.show_error("There are no projects yet.".into(), cx);
                } else {
                    this.open_menu(None, rows, point(at.x, at.y + px(12.)), false, window, cx);
                }
            })
            .ok();
    });
    let delete = button("prints-picked-delete", "trash", Some("Delete\u{2026}".into()), format!("Delete the {}. Deleted prints are kept for 7 days", count(picked))).on_click(move |_, window, cx| {
        delete_app
            .update(cx, |this, cx| {
                let ids = this.picked_prints();
                this.ask_delete_prints(ids, window, cx);
            })
            .ok();
    });
    let clear = button("prints-picked-clear", "xmark", None, "Pick none (Esc)".into()).on_click(move |_, _, cx| {
        clear_app
            .update(cx, |this, cx| {
                this.clear_picked_prints(cx);
            })
            .ok();
    });
    Some(
        h_flex()
            .id("prints-picked-bar")
            .w_full()
            .gap(px(4.))
            .px(px(8.))
            .py(px(4.))
            .mb(px(4.))
            .items_center()
            .rounded(px(7.))
            .bg(style::primary_opacity(cx, 0.05))
            .text_color(secondary)
            .child(div().flex_none().pl(px(4.)).text_size(px(style::size::SMALL)).text_color(cx.theme().foreground).child(format!("{picked} picked")))
            .child(div().flex_1())
            .child(projects)
            .child(delete)
            .child(clear)
            .into_any_element(),
    )
}

/// Esc anywhere lets the picked Prints go, unless a menu or a dialog is up
/// for it to close first.
pub(crate) fn install(app: WeakEntity<Threadbear>, cx: &mut App) {
    cx.intercept_keystrokes(move |event, window, cx| {
        let k = &event.keystroke;
        if k.key != "escape" || k.modifiers.modified() {
            return;
        }
        if window.has_active_dialog(cx) {
            return;
        }
        app.update(cx, |this, cx| {
            if this.sidebar.menu.is_none() {
                this.clear_picked_prints(cx);
            }
        })
        .ok();
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::{count, naming, picked_after, Pick};

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn command_click_picks_and_lets_go() {
        let order = ids(&["a", "b", "c", "d"]);
        let (now, anchor) = picked_after(&[], None, "b", &order, Pick::Toggle);
        assert_eq!((now.clone(), anchor.as_deref()), (ids(&["b"]), Some("b")));
        let (now, _) = picked_after(&now, Some("b"), "d", &order, Pick::Toggle);
        assert_eq!(now, ids(&["b", "d"]));
        let (now, _) = picked_after(&now, Some("d"), "b", &order, Pick::Toggle);
        assert_eq!(now, ids(&["d"]));
    }

    #[test]
    fn shift_click_picks_a_run_either_way() {
        let order = ids(&["a", "b", "c", "d", "e"]);
        assert_eq!(picked_after(&ids(&["b"]), Some("b"), "d", &order, Pick::Range).0, ids(&["b", "c", "d"]));
        assert_eq!(picked_after(&ids(&["d"]), Some("d"), "a", &order, Pick::Range).0, ids(&["a", "b", "c", "d"]));
        // Nothing picked yet: just that one, and it's where the next run starts.
        assert_eq!(picked_after(&[], None, "c", &order, Pick::Range), (ids(&["c"]), Some("c".to_owned())));
        // The anchor stays, so a second Shift-click moves the run's far end.
        let (now, anchor) = picked_after(&ids(&["b", "c", "d"]), Some("b"), "c", &order, Pick::Range);
        assert_eq!((now, anchor.as_deref()), (ids(&["b", "c"]), Some("b")));
    }

    #[test]
    fn a_plain_click_lets_them_all_go() {
        assert_eq!(picked_after(&ids(&["a", "b"]), Some("a"), "c", &ids(&["a", "b", "c"]), Pick::Plain), (vec![], None));
    }

    #[test]
    fn the_confirm_names_them() {
        assert_eq!(count(1), "1 print");
        assert_eq!(count(3), "3 prints");
        assert_eq!(naming(&ids(&["One", "Two"])), "\u{201C}One\u{201D}, \u{201C}Two\u{201D}");
        assert!(naming(&ids(&["1", "2", "3", "4", "5", "6", "7"])).ends_with(" and 2 more"));
    }
}
