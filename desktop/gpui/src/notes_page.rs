//! Private notes. Shares the Swift app's encrypted files, never the RPC client.
use crate::{app::Threadbear, style, icons};
use crate::style::px;
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Sizable as _};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::input::{Editor, EditorState, Input, InputState, InputEvent};
use gpui_kit::prelude::*;
use gpui_kit::*;
use threadbear_core::notes::Store;
use serde_json::{json, Value};

pub(crate) struct NotesPage {
    pub open: bool,
    pub store: Store,
    selected: Option<String>,
    editor: Option<Entity<EditorState>>,
    title: Option<Entity<InputState>>,
    subscriptions: Vec<Subscription>,
    error: Option<String>,
    saved: bool,
}
impl NotesPage {
    pub fn new() -> Self {
        Self { open: false, store: Store::in_home(&threadbear_client::config::RunnerConfig::from_env().home), selected: None, editor: None, title: None, subscriptions: vec![], error: None, saved: true }
    }
    pub fn dump(&self, folded: bool, cx: &App) -> Value {
        json!({"open":self.open,"folded":folded,"saved":self.saved,"error":self.error,
            "title":self.title.as_ref().map(|t|t.read(cx).value().to_string()),
            "body":self.editor.as_ref().map(|e|e.read(cx).value().to_string()),
            "count":self.store.notes.len()})
    }
}
impl Threadbear {
    pub(crate) fn show_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.saved.show_notes == Some(false) { return; }
        self.notes.open = true;
        window.focus(&self.sidebar.focus, cx);
        crate::web_panels::keyboard_to_app(window);
        self.notes.error = self.notes.store.load().err().map(|e| e.text().to_owned());
        if self.notes.selected.is_none() {
            let first = self.notes.store.sorted().first().map(|n|n.id.clone());
            if let Some(id) = first { self.select_note(id, window, cx); }
        }
        if let Some(editor) = &self.notes.editor { window.focus(&editor.read(cx).focus_handle(cx), cx); }
        cx.notify();
    }
    pub(crate) fn close_notes(&mut self, cx: &mut Context<Self>) {
        self.notes.open = false;
        cx.notify();
    }
    fn select_note(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(note) = self.notes.store.get(&id).cloned() else { return };
        self.notes.subscriptions.clear();
        let editor = cx.new(|cx| {
            let mut s = EditorState::new(window, cx).soft_wrap(true).line_number(true).folding(false).indent_guides(false).language("markdown");
            s.set_value(note.body, window, cx);
            s
        });
        let title = cx.new(|cx| { let mut s = InputState::new(window,cx); s.set_value(note.title, window, cx); s });
        self.notes.selected = Some(id);
        self.notes.subscriptions.push(cx.subscribe(&editor, |this, e, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let body = e.read(cx).value().to_string();
                if let Some(id) = this.notes.selected.clone() {
                    this.notes.store.set_body(&id, &body);
                    this.save_note(&id, cx);
                }
            }
        }));
        self.notes.subscriptions.push(cx.subscribe_in(&title, window, |this, e, event: &InputEvent, window, cx| {
            if matches!(event, InputEvent::Change | InputEvent::Blur | InputEvent::PressEnter { .. }) {
                if let Some(id) = this.notes.selected.clone() {
                    this.notes.store.rename(&id, &e.read(cx).value().to_string());
                    this.save_note(&id, cx);
                    if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                        if let Some(title) = this.notes.store.get(&id).map(|n|n.title.clone()) { e.update(cx,|s,cx|s.set_value(title,window,cx)); }
                    }
                }
            }
        }));
        window.focus(&editor.read(cx).focus_handle(cx), cx);
        self.notes.editor = Some(editor);
        self.notes.title = Some(title);
        cx.notify();
    }
    fn save_note(&mut self, id: &str, cx: &mut Context<Self>) {
        match self.notes.store.write(id) {
            Ok(()) => { self.notes.saved = true; self.notes.error = None; }
            Err(e) => { self.notes.saved = false; self.notes.error = Some(format!("Couldn't save your note: {e}")); }
        }
        cx.notify();
    }
    fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.notes.store.create() { self.select_note(id.clone(), window, cx); self.save_note(&id,cx); }
    }
    fn fold_notes(&mut self, cx: &mut Context<Self>) {
        self.settings.saved.notes_list_collapsed = Some(self.settings.saved.notes_list_collapsed != Some(true));
        self.save_settings_soon(cx); cx.notify();
    }
    fn delete_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.notes.selected.clone() else { return };
        let answer = window.prompt(PromptLevel::Warning,"Delete this note?",Some("This can't be undone."), &["Delete", "Cancel"], cx);
        cx.spawn_in(window, async move |this, cx| {
            if matches!(answer.await, Ok(0)) {
                this.update_in(cx, |this, window, cx| {
                    if let Err(e) = this.notes.store.delete(&id) {
                        this.notes.error = Some(format!("Couldn't delete your note: {e}")); cx.notify(); return;
                    }
                    this.notes.selected = None; this.notes.editor = None; this.notes.title = None; this.notes.subscriptions.clear();
                    if let Some(next) = this.notes.store.sorted().first().map(|n|n.id.clone()) { this.select_note(next,window,cx); }
                    cx.notify();
                }).ok();
            }
        }).detach();
    }
    pub(crate) fn render_notes(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let secondary = style::secondary_text(cx);
        let folded = self.settings.saved.notes_list_collapsed == Some(true);
        let control = |id: &'static str, symbol: &'static str, tip: &'static str| div().id(id).test_support().role(Role::Button).aria_label(tip).p(px(7.)).rounded(px(5.)).hover(|s|s.bg(style::sidebar_hover(cx))).tip(tip).child(icons::symbol(symbol).with_size(px(15.)));
        let header = h_flex().gap(px(8.)).p(px(10.)).border_b_1().border_color(style::hairline(cx))
            .child(control("notes-fold", "sidebar.left", "Show or hide the notes list").on_click(cx.listener(|this,_,_,cx|this.fold_notes(cx))))
            .child(div().child("Notes"))
            .child(div().id("notes-privacy").flex_1().tip("Encrypted on this computer. Never sent to an agent, phone or website.").child(icons::symbol("lock").with_size(px(11.))))
            .when(self.notes.store.is_open(), |s| s.child(control("notes-new", "square.and.pencil", "New note").on_click(cx.listener(|this,_,w,cx|this.new_note(w,cx)))))
            .child(control("notes-close", "xmark", "Close Notes").on_click(cx.listener(|this,_,_,cx|this.close_notes(cx))));
        let rows: Vec<_> = self.notes.store.sorted().iter().map(|note| {
            let id = note.id.clone();
            let menu_id = id.clone();
            let weak = cx.entity().downgrade();
            v_flex().id(SharedString::from(format!("note-{}",note.id))).p(px(12.)).gap(px(4.)).w_full()
                .when(self.notes.selected.as_ref() == Some(&note.id), |s|s.bg(style::sidebar_hover(cx)))
                .tip("Open note").child(div().child(note.title.clone()))
                .child(div().text_size(px(12.)).text_color(secondary).child(note.preview().unwrap_or("Empty note").to_owned()))
                .on_click(cx.listener(move |this,_,w,cx|this.select_note(id.clone(),w,cx)))
                .context_menu(move |menu,_,_| {
                    let (weak,id)=(weak.clone(),menu_id.clone());
                    menu.item(PopupMenuItem::new("Delete note").on_click(move |_,window,cx| {
                        weak.update(cx,|this,cx| { this.select_note(id.clone(),window,cx); this.delete_note(window,cx); }).ok();
                    }))
                }).into_any_element()
        }).collect();
        let list = (!folded).then(||v_flex().id("notes-list").w(px(230.)).h_full().overflow_y_scroll().border_r_1().border_color(style::hairline(cx)).children(rows));
        let warning = (self.notes.store.unreadable > 0).then(||format!("{} notes couldn't be opened. Their files have been left untouched.",self.notes.store.unreadable));
        let content = match (&self.notes.editor,&self.notes.title) {
            (Some(editor),Some(title)) => v_flex().flex_1().min_w_0().h_full()
                .child(h_flex().p(px(12.)).gap(px(10.)).child(div().id("note-title").flex_1().tip("Rename note").child(Input::new(title).appearance(false).bordered(false)))
                    .child(control("notes-delete","trash","Delete note").on_click(cx.listener(|this,_,w,cx|this.delete_note(w,cx)))))
                .child(div().flex_1().min_h_0().child(Editor::new(editor).h_full().appearance(false).bordered(false).font_family(cx.theme().mono_font_family.clone())))
                .into_any_element(),
            _ => v_flex().flex_1().h_full().items_center().justify_center().gap(px(12.)).child("Your private notes")
                .child(div().text_color(secondary).child("Create a note for ideas, lists and things to remember."))
                .when(self.notes.store.is_open(),|s|s.child(div().id("notes-first").test_support().role(Role::Button).aria_label("New note").p(px(8.)).tip("New note").child("New Note").on_click(cx.listener(|this,_,w,cx|this.new_note(w,cx)))))
                .into_any_element(),
        };
        v_flex().id("notes-page").size_full().capture_key_down(cx.listener(|this,ev: &KeyDownEvent,_,cx| { if ev.keystroke.key == "escape" { this.close_notes(cx); cx.stop_propagation(); } }))
            .child(header).children(warning.map(|e|div().p(px(12.)).child(e))).children(self.notes.error.clone().map(|e| div().p(px(12.)).child(e)))
            .child(h_flex().flex_1().min_h_0().children(list).child(content)).into_any_element()
    }
    #[cfg(feature="automation")]
    pub(crate) fn notes_command(&mut self, cmd: &Value, window: &mut Window, cx: &mut Context<Self>) -> Result<Value,String> {
        if cmd["open"] == true { self.show_notes(window,cx); }
        if cmd["close"] == true { self.close_notes(cx); }
        if cmd["new"] == true { self.new_note(window,cx); }
        if cmd["fold"] == true { self.fold_notes(cx); }
        if let Some(title) = cmd["title"].as_str() {
            if let Some(id) = self.notes.selected.clone() {
                self.notes.store.rename(&id,title); self.save_note(&id,cx);
                if let Some(field) = &self.notes.title { field.update(cx,|s,cx|s.set_value(title,window,cx)); }
            }
        }
        if let Some(body) = cmd["body"].as_str() {
            if let Some(editor) = &self.notes.editor {
                self.notes.saved = false;
                editor.update(cx,|s,cx| { s.select_all(window,cx); s.replace(body.to_owned(),window,cx); });
            }
        }
        cx.notify(); Ok(json!("ok"))
    }
}
