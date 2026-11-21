// Tasks' Linear-style keys, their quick picker, and Favorites (tasks_page.rs).
//
// The keys only work on the list or the board, with the page itself focused
// (not a text field), and only for what Tasks has: a new task, status,
// priority, due date, delete, rename, the description and favorites. Linear's
// assignee, labels, estimate, template and reminder keys have nothing here to
// change, so they're left out.
//
// The app's own shortcuts are key bindings, and GPUI runs bindings before a
// view's key handler (Window::dispatch_key_event), so a key the app binds,
// now or in a later update from Mike, always wins over one of these.
use super::*;
use threadbear_core::task_dates;

/// What a key does on the list or board, as Linear's keys for an issue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum TaskKey {
    /// C
    NewTask,
    /// S
    Status,
    /// P
    Priority,
    /// Shift-D
    Due,
    /// Cmd-Shift-D
    ClearDue,
    /// Cmd-Backspace
    Delete,
    /// Shift-R
    Rename,
    /// Ctrl-Shift-I
    Description,
    /// Option-F
    Favorite,
}

pub(super) fn task_key(key: &str, m: &Modifiers) -> Option<TaskKey> {
    if m.function { return None; }
    Some(match (key, m.platform, m.control, m.alt, m.shift) {
        ("c", false, false, false, false) => TaskKey::NewTask,
        ("s", false, false, false, false) => TaskKey::Status,
        ("p", false, false, false, false) => TaskKey::Priority,
        ("d", false, false, false, true) => TaskKey::Due,
        ("d", true, false, false, true) => TaskKey::ClearDue,
        ("backspace", true, false, false, false) => TaskKey::Delete,
        ("r", false, false, false, true) => TaskKey::Rename,
        ("i", false, true, false, true) => TaskKey::Description,
        ("f", false, false, true, false) => TaskKey::Favorite,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PickKind { Status, Priority, Due }

/// The quick picker S, P and Shift-D open over the list or board.
pub(super) struct QuickPick {
    pub kind: PickKind,
    pub task: String,
    pub query: String,
    pub highlight: usize,
}

#[derive(Clone)]
enum PickApply { Status(String), Priority(Option<&'static str>), Due(Option<String>), PickDate }

struct PickItem {
    id: String,
    label: String,
    detail: Option<String>,
    icon: Option<Icon>,
    number: Option<char>,
    checked: bool,
    apply: PickApply,
}

/// Items whose name has the query in it, any case.
fn matches(label: &str, query: &str) -> bool {
    query.is_empty() || label.to_lowercase().contains(&query.to_lowercase())
}

impl TasksPage {
    /// The task a key acts on: the one under the pointer, else the selected one.
    fn key_target(&self) -> Option<TaskRow> {
        let id = self.hovered_task.as_ref().filter(|id| self.rows.iter().any(|r| &r.id == *id)).or(self.selected_task.as_ref())?;
        self.rows.iter().find(|r| &r.id == id).cloned()
    }

    /// The pointer came onto a row or card, or left it. Nothing is drawn
    /// differently, so the page isn't redrawn.
    pub(super) fn hover_task(&mut self, id: &str, over: bool) {
        if over {
            self.hovered_task = Some(id.to_string());
        } else if self.hovered_task.as_deref() == Some(id) {
            self.hovered_task = None;
        }
    }

    /// A Linear-style key on the list or board. True when it was used.
    pub(super) fn task_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.busy || self.quick_pick.is_some() || !self.focus.is_focused(window) { return false; }
        let Some(key) = task_key(event.keystroke.key.as_str(), &event.keystroke.modifiers) else { return false };
        // On a task's page only Option-F, for its star.
        if self.editor_open {
            let Some(row) = self.editing.as_ref().and_then(|id| self.rows.iter().find(|r| &r.id == id)).cloned() else { return false };
            if key != TaskKey::Favorite { return false; }
            self.toggle_favorite(&row, cx);
            return true;
        }
        if key == TaskKey::NewTask {
            self.new_task_key(window, cx);
            return true;
        }
        let Some(row) = self.key_target() else { return false };
        self.selected_task = Some(row.id.clone());
        match key {
            TaskKey::Status => self.open_quick_pick(PickKind::Status, &row, cx),
            TaskKey::Priority => self.open_quick_pick(PickKind::Priority, &row, cx),
            TaskKey::Due => self.open_quick_pick(PickKind::Due, &row, cx),
            TaskKey::ClearDue => self.clear_due(&row, cx),
            TaskKey::Delete => self.ask_delete(row.id.clone(), window, cx),
            TaskKey::Rename => {
                self.open_editor(&row, false, window, cx);
                if self.editor_open {
                    self.title.update(cx, |f, cx| f.focus(window, cx));
                    crate::web_panels::keyboard_to_app(window);
                }
            }
            TaskKey::Description => self.open_editor(&row, true, window, cx),
            TaskKey::Favorite => self.toggle_favorite(&row, cx),
            TaskKey::NewTask => {}
        }
        cx.notify();
        true
    }

    /// C: the quick-add box on the list, a new card in Building on the board.
    fn new_task_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view_name() == "list" {
            self.focus_composer(window, cx);
            return;
        }
        let group = if !self.by_project() {
            "all".to_string()
        } else {
            self.key_target().map(|r| project_key(&r.project)).or_else(|| self.groups(&self.visible()).into_iter().next()).unwrap_or_else(|| "none".into())
        };
        self.adding_in = Some((group, "todo".into()));
        self.column_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
            input.focus(window, cx);
        });
        crate::web_panels::keyboard_to_app(window);
        cx.notify();
    }

    fn clear_due(&mut self, row: &TaskRow, cx: &mut Context<Self>) {
        let Some(day) = row.due_date.clone() else { return };
        let mut undo = json!({"id": row.id, "dueDate": day});
        if let Some(time) = &row.due_time { undo["dueTime"] = json!(time); }
        self.patch(&row.id, json!({"dueDate": Value::Null}), Some((format!("Removed the due date: {}", row.title), undo)), cx);
    }

    /// Adds a task to Favorites, or takes it off. It stays where it is too.
    pub(super) fn toggle_favorite(&mut self, row: &TaskRow, cx: &mut Context<Self>) {
        let on = !row.favorite;
        let text = if on { format!("Added to Favorites: {}", row.title) } else { format!("Removed from Favorites: {}", row.title) };
        self.patch(&row.id, json!({"favorite": on}), Some((text, json!({"id": row.id, "favorite": !on}))), cx);
    }

    // MARK: the quick picker

    fn open_quick_pick(&mut self, kind: PickKind, row: &TaskRow, cx: &mut Context<Self>) {
        let items = self.pick_items(kind, row, cx);
        let highlight = items.iter().position(|i| i.checked).unwrap_or(0);
        self.quick_pick = Some(QuickPick { kind, task: row.id.clone(), query: String::new(), highlight });
        cx.notify();
    }

    pub(super) fn close_quick_pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.quick_pick = None;
        window.focus(&self.focus, cx);
        crate::web_panels::keyboard_to_app(window);
        cx.notify();
    }

    fn pick_items(&self, kind: PickKind, row: &TaskRow, cx: &App) -> Vec<PickItem> {
        let digit = |i: usize| char::from_digit(i as u32, 10);
        match kind {
            PickKind::Status => {
                let order = if self.view_name() == "list" { self.list_order() } else { self.board_order() };
                order.into_iter().enumerate().map(|(i, s)| PickItem {
                    id: format!("status-{s}"),
                    label: self.column_name(&s),
                    detail: None,
                    icon: Some(self.column_icon(&s, cx)),
                    number: if i < 9 { digit(i + 1) } else { None },
                    checked: column_id(row) == s,
                    apply: PickApply::Status(s),
                }).collect()
            }
            PickKind::Priority => {
                let mut choices: Vec<(Option<&'static str>, &'static str)> = vec![(None, "No priority")];
                choices.extend(task_dates::PRIORITIES.iter().map(|(id, name)| (Some(*id), *name)));
                choices.into_iter().enumerate().map(|(i, (value, name))| PickItem {
                    id: format!("priority-{}", value.unwrap_or("none")),
                    label: name.to_string(),
                    detail: None,
                    icon: Some(priority_icon(value, 14., cx)),
                    number: digit(i),
                    checked: row.priority.as_deref() == value,
                    apply: PickApply::Priority(value),
                }).collect()
            }
            PickKind::Due => {
                let current = row.due_date.as_deref().and_then(task_dates::parse);
                let mut items: Vec<PickItem> = task_dates::quick_picks(detail::today()).into_iter().map(|(label, day)| PickItem {
                    id: format!("due-{}", label.to_lowercase().replace(' ', "-")),
                    label: label.to_string(),
                    detail: Some(task_dates::long_label(day)),
                    icon: Some(icons::symbol("calendar").with_size(px(13.)).text_color(style::secondary_label(cx))),
                    number: None,
                    checked: current == Some(day),
                    apply: PickApply::Due(Some(task_dates::format(day))),
                }).collect();
                items.push(PickItem { id: "due-pick".into(), label: "Pick a date…".into(), detail: None, icon: Some(icons::symbol("calendar.badge.clock").with_size(px(13.)).text_color(style::secondary_label(cx))), number: None, checked: false, apply: PickApply::PickDate });
                if row.due_date.is_some() {
                    items.push(PickItem { id: "due-clear".into(), label: "Remove due date".into(), detail: None, icon: Some(icons::symbol("xmark").with_size(px(12.)).text_color(style::secondary_label(cx))), number: None, checked: false, apply: PickApply::Due(None) });
                }
                for (i, item) in items.iter_mut().enumerate() {
                    item.number = if i < 9 { digit(i + 1) } else { None };
                }
                items
            }
        }
    }

    fn shown_items(&self, cx: &App) -> Vec<PickItem> {
        let Some(pick) = &self.quick_pick else { return vec![] };
        let Some(row) = self.rows.iter().find(|r| r.id == pick.task) else { return vec![] };
        self.pick_items(pick.kind, row, cx).into_iter().filter(|i| matches(&i.label, &pick.query)).collect()
    }

    fn apply_pick(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.shown_items(cx);
        let Some(task) = self.quick_pick.as_ref().map(|p| p.task.clone()) else { return };
        let Some(item) = items.into_iter().nth(index) else { return };
        let Some(row) = self.rows.iter().find(|r| r.id == task).cloned() else { return self.close_quick_pick(window, cx) };
        self.close_quick_pick(window, cx);
        match item.apply {
            PickApply::Status(s) => self.set_field(&task, "status", json!(s), cx),
            PickApply::Priority(v) => self.set_field(&task, "priority", v.map_or(Value::Null, |v| json!(v)), cx),
            PickApply::Due(Some(day)) => self.set_field(&task, "dueDate", json!(day), cx),
            PickApply::Due(None) => self.clear_due(&row, cx),
            PickApply::PickDate => self.pick_due_date(&row, window, cx),
        }
    }

    /// Keys while the picker is open: type to filter, arrows and Return, or a
    /// row's number. Esc goes through CloseTasks.
    pub(super) fn quick_pick_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let m = &event.keystroke.modifiers;
        if m.platform || m.control { return; }
        let count = self.shown_items(cx).len();
        let Some(pick) = self.quick_pick.as_mut() else { return };
        match event.keystroke.key.as_str() {
            "down" => pick.highlight = if count == 0 { 0 } else { (pick.highlight + 1) % count },
            "up" => pick.highlight = if count == 0 { 0 } else { (pick.highlight + count - 1) % count },
            "enter" => { let i = pick.highlight; self.apply_pick(i, window, cx); return; }
            "backspace" => { pick.query.pop(); pick.highlight = 0; }
            key => {
                let typed = event.keystroke.key_char.clone().filter(|c| !c.chars().any(char::is_control));
                if pick.query.is_empty() && !m.alt && key.len() == 1 && key.chars().all(|c| c.is_ascii_digit()) {
                    let digit = key.chars().next();
                    let items = self.shown_items(cx);
                    if let Some(index) = items.iter().position(|i| i.number == digit) { self.apply_pick(index, window, cx); }
                    return;
                }
                let Some(typed) = typed else { return };
                pick.query.push_str(&typed);
                pick.highlight = 0;
            }
        }
        cx.notify();
    }

    pub(super) fn quick_pick_view(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pick = self.quick_pick.as_ref()?;
        let row = self.rows.iter().find(|r| r.id == pick.task)?;
        let (placeholder, key) = match pick.kind {
            PickKind::Status => ("Change status…", "S"),
            PickKind::Priority => ("Set priority…", "P"),
            PickKind::Due => ("Set due date…", "⇧D"),
        };
        let items = self.shown_items(cx);
        let (hover, tertiary) = (style::hover_fill(cx), style::tertiary_label(cx));
        let query = if pick.query.is_empty() {
            h_flex().child(div().w(px(1.5)).h(px(18.)).bg(style::accent(cx))).child(div().text_color(tertiary).child(placeholder))
        } else {
            h_flex().child(pick.query.clone()).child(div().w(px(1.5)).h(px(18.)).bg(style::accent(cx)))
        };
        let mut list = v_flex().p(px(6.)).gap(px(1.));
        if items.is_empty() {
            list = list.child(div().px(px(10.)).py(px(8.)).text_size(px(13.)).text_color(tertiary).child("No matches"));
        }
        for (index, item) in items.into_iter().enumerate() {
            let lit = index == pick.highlight;
            list = list.child(
                h_flex().id(SharedString::from(format!("quick-pick-{}", item.id))).role(Role::MenuItem).aria_label(SharedString::from(item.label.clone()))
                    .h(px(34.)).px(px(10.)).gap(px(10.)).rounded(px(7.)).cursor_pointer().text_size(px(14.))
                    .when(lit, |d| d.bg(hover))
                    .hover(move |s| s.bg(hover))
                    .on_click(cx.listener(move |this, _, window, cx| this.apply_pick(index, window, cx)))
                    .child(div().w(px(16.)).flex().justify_center().children(item.icon))
                    .child(div().flex_1().min_w_0().truncate().child(item.label.clone().test_text(item.label.clone())))
                    .children(item.detail.map(|d| div().text_size(px(12.)).text_color(tertiary).child(d)))
                    .child(div().w(px(14.)).children(item.checked.then(|| icons::symbol("checkmark").with_size(px(12.)).text_color(style::secondary_label(cx)))))
                    .child(div().w(px(12.)).text_size(px(12.)).text_color(tertiary).children(item.number.map(|n| n.to_string()))),
            );
        }
        let card = v_flex().id("tasks-quick-pick").test_support().occlude().w(px(420.)).max_w_full().rounded(px(12.)).bg(cx.theme().popover).border_1().border_color(cx.theme().border).shadow_lg().text_color(style::label_text(cx))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().px(px(14.)).pt(px(10.)).text_size(px(12.)).text_color(tertiary).truncate().child(row.title.clone()))
            .child(h_flex().px(px(14.)).py(px(10.)).gap(px(10.)).border_b_1().border_color(style::hairline(cx)).text_size(px(15.))
                .child(div().flex_1().min_w_0().child(query))
                .child(div().px(px(6.)).py(px(1.)).rounded(px(5.)).border_1().border_color(style::hairline(cx)).text_size(px(11.)).text_color(style::secondary_label(cx)).child(key)))
            .child(list);
        Some(
            div().id("tasks-quick-pick-backdrop").absolute().inset_0().flex().justify_center().pt(px(110.)).bg(gpui_kit::black().opacity(0.18))
                .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_quick_pick(window, cx)))
                .child(card)
                .into_any_element(),
        )
    }

    // MARK: Favorites

    /// Favorite tasks, in the page's sort, done ones only while done shows.
    pub(super) fn favorite_rows(&self, rows: &[TaskRow], board: bool) -> Vec<TaskRow> {
        rows.iter().filter(|r| r.favorite && (board || self.show_done || r.status != "done")).cloned().collect()
    }

    /// The Favorites section at the top of the list. It can fold, but can't
    /// be dragged or dropped on: tasks stay in their own section too.
    pub(super) fn favorites_section(&self, rows: &[TaskRow], cx: &mut Context<Self>) -> Option<AnyElement> {
        let items = self.favorite_rows(rows, false);
        if items.is_empty() { return None; }
        let key = "favorites".to_string();
        let label = h_flex().gap(px(8.)).child(favorite_icon(true, 14., cx)).child("Favorites").into_any_element();
        let header = h_flex().id("section-favorites").h(px(32.)).px(px(8.)).rounded(px(6.)).bg(style::hover_fill(cx)).text_size(px(13.)).font_weight(FontWeight::MEDIUM)
            .child(self.fold(key.clone(), "Favorites".into(), label, items.len(), cx));
        let mut section = v_flex().id("favorites").test_support().mb(px(14.)).rounded(px(8.)).child(header);
        if !self.collapsed.contains(&key) {
            section = section.child(div().h(px(10.)).flex_none());
            for task in &items {
                section = section.child(self.row(task, FAVORITES, cx));
            }
        }
        Some(div().mb(px(18.)).child(section).into_any_element())
    }

    /// The Favorites column, first on the board. Its cards open and have the
    /// ⋯ menu but don't drag: the task moves in its own column.
    pub(super) fn favorites_column(&self, rows: &[TaskRow], cx: &mut Context<Self>) -> Option<AnyElement> {
        let items = self.favorite_rows(rows, true);
        if items.is_empty() { return None; }
        let wrap = self.wrap_columns;
        let column_key = "column:favorites".to_string();
        let header = h_flex().id("column-head-favorites").h(px(34.)).px(px(6.)).gap(px(8.)).text_size(px(13.)).font_weight(FontWeight::MEDIUM)
            .child(favorite_icon(true, 14., cx))
            .child(div().px(px(4.)).child("Favorites"))
            .child(div().text_color(style::tertiary_label(cx)).child(items.len().to_string()));
        let mut cards = v_flex().id("cards-favorites").min_h_0()
            .when(!wrap, |d| d.max_h(px((self.viewport_height - 210.).max(140.))).overflow_y_scroll().track_scroll(&self.pane_scroll(&column_key)));
        for task in &items {
            cards = cards.child(self.card(task, FAVORITES, cx));
        }
        Some(v_flex().id("column-favorites").test_support().flex_none().w(px(280.)).min_h(px(160.)).p(px(6.)).rounded(px(10.)).bg(style::hover_fill(cx)).child(header).child(cards).into_any_element())
    }
}

/// The group name a row or card has under Favorites.
pub(super) const FAVORITES: &str = "favorites";

pub(super) fn favorite_icon(on: bool, size: f32, cx: &App) -> Icon {
    if on { icons::symbol("star.fill").with_size(px(size)).text_color(Hsla::from(rgb(0xe2a336))) } else { icons::symbol("star").with_size(px(size)).text_color(style::secondary_label(cx)) }
}

#[cfg(test)]
mod tests {
    // Not super::*: gpui's own #[test] would come with it.
    use super::{matches, task_key, TaskKey};
    use gpui_kit::Modifiers;

    fn mods(cmd: bool, ctrl: bool, alt: bool, shift: bool) -> Modifiers {
        Modifiers { platform: cmd, control: ctrl, alt, shift, function: false }
    }

    #[test]
    fn linear_keys_map_to_what_tasks_has() {
        let none = mods(false, false, false, false);
        assert_eq!(task_key("c", &none), Some(TaskKey::NewTask));
        assert_eq!(task_key("s", &none), Some(TaskKey::Status));
        assert_eq!(task_key("p", &none), Some(TaskKey::Priority));
        assert_eq!(task_key("d", &mods(false, false, false, true)), Some(TaskKey::Due));
        assert_eq!(task_key("d", &mods(true, false, false, true)), Some(TaskKey::ClearDue));
        assert_eq!(task_key("backspace", &mods(true, false, false, false)), Some(TaskKey::Delete));
        assert_eq!(task_key("r", &mods(false, false, false, true)), Some(TaskKey::Rename));
        assert_eq!(task_key("i", &mods(false, true, false, true)), Some(TaskKey::Description));
        assert_eq!(task_key("f", &mods(false, false, true, false)), Some(TaskKey::Favorite));
    }

    #[test]
    fn other_keys_and_extra_modifiers_do_nothing() {
        // Linear's keys for things Tasks doesn't have.
        for key in ["a", "i", "l", "h", "e", "t"] {
            assert_eq!(task_key(key, &mods(false, false, false, false)), None, "{key}");
        }
        assert_eq!(task_key("e", &mods(false, false, false, true)), None);
        assert_eq!(task_key("h", &mods(false, false, false, true)), None);
        // The app's own Cmd keys (Cmd-S, Cmd-P, Cmd-C) aren't these.
        assert_eq!(task_key("s", &mods(true, false, false, false)), None);
        assert_eq!(task_key("p", &mods(true, false, false, false)), None);
        assert_eq!(task_key("c", &mods(true, false, false, false)), None);
        assert_eq!(task_key("d", &mods(false, false, false, false)), None);
        assert_eq!(task_key("backspace", &mods(false, false, false, false)), None);
    }

    #[test]
    fn none_clash_with_the_apps_default_shortcuts() {
        use threadbear_core::shortcuts::ShortcutCommand;
        let mine = [("c", mods(false, false, false, false)), ("s", mods(false, false, false, false)), ("p", mods(false, false, false, false)),
            ("d", mods(false, false, false, true)), ("d", mods(true, false, false, true)), ("backspace", mods(true, false, false, false)),
            ("r", mods(false, false, false, true)), ("i", mods(false, true, false, true)), ("f", mods(false, false, true, false))];
        for command in ShortcutCommand::ALL {
            let Some(key) = command.default_key() else { continue };
            let chord = key.chord(true);
            for (k, m) in &mine {
                let mut ours = String::new();
                if m.control { ours.push_str("ctrl-"); }
                if m.alt { ours.push_str("alt-"); }
                if m.shift { ours.push_str("shift-"); }
                if m.platform { ours.push_str("cmd-"); }
                ours.push_str(k);
                assert!(!same_chord(&chord, &ours), "{command:?} uses {chord}, as Tasks' {ours} does");
            }
        }
    }

    /// Two GPUI chords with the same modifiers and key, in any order.
    fn same_chord(a: &str, b: &str) -> bool {
        let parts = |s: &str| { let mut p: Vec<String> = s.split('-').map(String::from).collect(); p.sort(); p };
        parts(a) == parts(b)
    }

    #[test]
    fn the_picker_filters_by_name() {
        assert!(matches("In Progress", ""));
        assert!(matches("In Progress", "prog"));
        assert!(!matches("Done", "prog"));
    }
}
