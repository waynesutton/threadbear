// Notes (the bar's Notes, ⌘6), a port of the Swift app's NotesPage and
// NoteEditor: your own notes over the main panel, as Settings is, with the
// thread and its draft left where they were underneath. The list of notes,
// newest first, down the left (it folds away for more room to write); the
// open note on the right, its title above a code editor's box with line
// numbers and basic Markdown coloured as you type.
//
// The notes are read and written by threadbear_core::notes, in the same
// sealed files the Swift app keeps, so both apps show the same notes. They
// never go to the runner, an agent, the phone or the website.
use crate::app::Threadbear;
use crate::highlights::{tipped, tipped_as};
use crate::style::{self, px};
use crate::tips::Tip as _;
use crate::icons;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Editor, EditorState, Enter, FoldRange, HighlightStyleResolver, Input, InputEdit, InputEvent, InputHighlighter, InputHighlighterFactory, InputState, Rope, TabSize};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _, Sizable as _, WindowExt as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;
use threadbear_core::notes::{self, Kind, Store};

/// The editor's language name, for the note highlighter below.
const NOTE_LANGUAGE: &str = "threadbear-note";
/// Typing is written half a second after it stops (`setBody`).
const SAVE_DELAY: Duration = Duration::from_millis(500);
/// The list's width at 100% (`.frame(width: 220)`).
const LIST_WIDTH: f32 = 220.;

actions!(notes_page, [CloseNotes]);

const PAGE_CONTEXT: &str = "NotesPage";

fn bind_keys(cx: &mut App) {
    static BOUND: std::sync::Once = std::sync::Once::new();
    // Esc leaves Notes, once the editor has nothing of its own to put away
    // (a menu, extra carets): it hands Esc on when it doesn't use it.
    BOUND.call_once(|| cx.bind_keys([KeyBinding::new("escape", CloseNotes, Some(PAGE_CONTEXT))]));
}

impl Threadbear {
    pub(crate) fn notes_shown(&self) -> bool {
        self.settings.notes_shown()
    }

    /// ⌘6 and the bar's button: Notes up, or put away if it's up.
    pub(crate) fn toggle_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.close_notes(cx) {
            self.open_notes(window, cx);
        }
    }

    /// Notes over the main panel, in place of Settings if that's up. Nothing
    /// while Show Notes is off, so a stale key can't bring it back.
    pub(crate) fn open_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.notes_shown() || self.notes_page.is_some() {
            return;
        }
        self.close_tasks(cx);
        self.close_settings(cx);
        let home = threadbear_client::config::RunnerConfig::from_env().home;
        let list_hidden = self.settings.saved.notes_list_hidden == Some(true);
        let app = cx.entity().downgrade();
        let page = cx.new(|cx| NotesPage::new(app, &home, list_hidden, window, cx));
        self.notes_page = Some(page);
        self.sync_menus(cx);
        cx.notify();
    }

    /// Notes put away, what's typed written first. False if it wasn't up.
    pub(crate) fn close_notes(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(page) = self.notes_page.take() else { return false };
        page.update(cx, |page, cx| page.flush(cx));
        // The keyboard back to the window. The note's editor can outlive the
        // page for a moment and keep the focus, which took every key after
        // ⌘6 or ⌘W closed Notes (only Esc, from inside, let go of it): so
        // the page's own handles are held until then, and lose it too.
        let held = page.read(cx).focus_handles(cx);
        let (window, focus) = (self.main_window, self.root_focus.clone());
        cx.defer(move |cx| {
            window
                .update(cx, |_, window, cx| {
                    if window.focused(cx).is_none() || held.iter().any(|h| h.is_focused(window)) {
                        window.focus(&focus, cx);
                        crate::web_panels::keyboard_to_app(window);
                    }
                })
                .ok();
        });
        self.sync_menus(cx);
        cx.notify();
        true
    }

    /// Settings > General's switch: off puts the page away and keeps the notes.
    pub(crate) fn set_notes_shown(&mut self, on: bool, cx: &mut Context<Self>) {
        self.settings.saved.notes_shown = (!on).then_some(false);
        if !on {
            self.close_notes(cx);
        }
        self.save_settings_soon(cx);
        self.sync_menus(cx);
        cx.notify();
    }

    pub(crate) fn render_notes_page(&self) -> AnyElement {
        match &self.notes_page {
            Some(page) => div().size_full().child(page.clone()).into_any_element(),
            None => div().into_any_element(),
        }
    }

    /// The dump's `notes`: whether it's up, the bar's button, and the page.
    pub(crate) fn notes_dump(&self, cx: &App) -> Value {
        let page = self.notes_page.as_ref().map(|p| p.read(cx));
        json!({
            "open": page.is_some(),
            "button": self.notes_shown(),
            "listed": page.map_or(vec![], |p| p.store.sorted().iter().map(|n| n.title.clone()).collect::<Vec<_>>()),
            "title": page.and_then(|p| p.open_note()).map(|n| n.title.clone()),
            "body": page.and_then(|p| p.selection.as_ref().map(|_| p.editor.read(cx).value().to_string())),
            "asking": page.and_then(|p| p.deleting.as_ref().map(|t| format!("Delete \u{201C}{t}\u{201D}?"))),
            "listHidden": page.map(|p| p.list_hidden),
            "problem": page.and_then(|p| p.store.problem.clone()),
        })
    }
}

/// The colours the highlighter paints with, from the theme as it's drawn.
#[derive(Clone, Copy, Default)]
struct Palette {
    accent: Hsla,
    secondary: Hsla,
    tertiary: Hsla,
    code: Hsla,
    code_background: Hsla,
    link: Hsla,
}

pub(crate) struct NotesPage {
    app: WeakEntity<Threadbear>,
    store: Store,
    selection: Option<String>,
    title: Entity<InputState>,
    editor: Entity<EditorState>,
    palette: Rc<RefCell<Palette>>,
    /// The note whose text waits to be written, and the wait.
    save_soon: Option<(String, Task<()>)>,
    /// The title of the note a delete is asking about.
    deleting: Option<String>,
    list_hidden: bool,
    scroll: ScrollHandle,
    focus: FocusHandle,
    _subs: Vec<Subscription>,
}

impl NotesPage {
    fn new(app: WeakEntity<Threadbear>, home: &std::path::Path, list_hidden: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        bind_keys(cx);
        let mut store = Store::new(home);
        store.load();
        let title = cx.new(|cx| InputState::new(window, cx).placeholder("Title"));
        let palette = Rc::new(RefCell::new(Palette::default()));
        let editor = cx.new(|cx| {
            // A code editor's box, as the Swift app's (Sublime Text's): line
            // numbers, and nothing closed, indented or folded for you.
            EditorState::new(window, cx)
                .soft_wrap(true)
                .line_number(true)
                .folding(false)
                .indent_guides(false)
                .auto_close(false)
                .smart_indent(false)
                .searchable(false)
                .tab_size(TabSize { tab_size: 4, hard_tabs: false })
                .language(NOTE_LANGUAGE)
        });
        let colours = palette.clone();
        let factory: InputHighlighterFactory = Rc::new(move |language: &str| {
            (language == NOTE_LANGUAGE).then(|| Box::new(NoteHighlighter { spans: vec![], palette: colours.clone() }) as Box<dyn InputHighlighter>)
        });
        editor.update(cx, |state, cx| state.set_highlighter_factory(factory, cx));
        let mut subs = vec![
            cx.subscribe(&editor, |this, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.body_typed(cx);
                }
            }),
            // Return or leaving the title renames the note (`onSubmit`, focus lost).
            cx.subscribe_in(&title, window, |this, _, ev: &InputEvent, window, cx| {
                if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    this.commit_title(cx);
                    this.show_title(window, cx);
                    if matches!(ev, InputEvent::PressEnter { .. }) {
                        this.editor.update(cx, |e, cx| e.focus(window, cx));
                        crate::web_panels::keyboard_to_app(window);
                    }
                }
            }),
        ];
        // What's typed and not yet written goes to disk on the way out. Local
        // files, written there and then.
        subs.push(cx.on_app_quit(|this, cx| {
            this.flush(cx);
            async {}
        }));
        let mut page = NotesPage { app, store, selection: None, title, editor, palette, save_soon: None, deleting: None, list_hidden, scroll: ScrollHandle::new(), focus: cx.focus_handle(), _subs: subs };
        let first = page.store.sorted().first().map(|n| n.id.clone());
        page.pick(first, window, cx);
        if page.selection.is_none() {
            window.focus(&page.focus, cx);
        }
        crate::web_panels::keyboard_to_app(window);
        page
    }

    /// Everything on the page that can hold the keyboard.
    fn focus_handles(&self, cx: &App) -> Vec<FocusHandle> {
        vec![self.focus.clone(), self.title.read(cx).focus_handle(cx), self.editor.read(cx).focus_handle(cx)]
    }

    fn open_note(&self) -> Option<&notes::Note> {
        self.store.get(self.selection.as_deref()?)
    }

    /// Another note in the editor, the last one's title and text written first.
    fn pick(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.flush(cx);
        self.selection = id.filter(|id| self.store.get(id).is_some());
        // Talking into the last note stops with it (voice/notes.rs).
        crate::voice::note_shown(self.selection.as_deref(), cx);
        let (title, body) = self.open_note().map_or((String::new(), String::new()), |n| (n.title.clone(), n.body.clone()));
        self.title.update(cx, |t, cx| t.set_value(title, window, cx));
        // Setting the text isn't typing: the subscription hears neither.
        self.editor.update(cx, |e, cx| {
            e.set_value(body, window, cx);
            e.set_selected_range(0..0, cx);
        });
        if self.selection.is_some() {
            self.editor.update(cx, |e, cx| e.focus(window, cx));
            crate::web_panels::keyboard_to_app(window);
        }
        cx.notify();
    }

    fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.flush(cx);
        if let Some(id) = self.store.create() {
            self.pick(Some(id), window, cx);
        }
    }

    /// Typed in the box: kept now, written half a second after it stops.
    fn body_typed(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selection.clone() else { return };
        let text = self.editor.read(cx).value().to_string();
        if !self.store.set_body(&id, &text) {
            return;
        }
        let wait = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_DELAY).await;
            this.update(cx, |this, _| this.write_pending()).ok();
        });
        self.save_soon = Some((id, wait));
        cx.notify();
    }

    fn write_pending(&mut self) {
        if let Some((id, _)) = self.save_soon.take() {
            self.store.save(&id);
        }
    }

    /// Anything waiting, written now: leaving the page, another note, quitting.
    fn flush(&mut self, cx: &mut Context<Self>) {
        self.write_pending();
        self.commit_title(cx);
    }

    /// The title as typed, cleaned and kept if it's new.
    fn commit_title(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selection.clone() else { return };
        let typed = self.title.read(cx).value().to_string();
        if self.store.rename(&id, &typed) {
            cx.notify();
        }
    }

    /// The title field back to the kept title (an empty one keeps the old name).
    fn show_title(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kept) = self.open_note().map(|n| n.title.clone()) else { return };
        if self.title.read(cx).value().as_ref() != kept {
            self.title.update(cx, |t, cx| t.set_value(kept, window, cx));
        }
    }

    /// Asks first: "It can't be brought back."
    fn ask_delete(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(title) = self.store.get(&id).map(|n| n.title.clone()) else { return };
        self.deleting = Some(title.clone());
        cx.notify();
        let me = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let (ok, closed, id) = (me.clone(), me.clone(), id.clone());
            alert
                .title(format!("Delete \u{201C}{title}\u{201D}?"))
                .description("It can't be brought back.")
                .button_props(DialogButtonProps::default().ok_text("Delete").show_cancel(true))
                .on_ok(move |_, window, cx| {
                    ok.update(cx, |this, cx| this.delete(&id, window, cx)).ok();
                    true
                })
                .on_close(move |_, _, cx| {
                    closed.update(cx, |this, cx| {
                        this.deleting = None;
                        cx.notify();
                    })
                    .ok();
                })
        });
    }

    fn delete(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.save_soon.as_ref().is_some_and(|(pending, _)| pending == id) {
            self.save_soon = None;
        }
        let next = self.store.sorted().into_iter().find(|n| n.id != id).map(|n| n.id.clone());
        self.store.delete(id);
        self.deleting = None;
        if self.selection.as_deref() == Some(id) {
            self.selection = None;
            self.pick(next, window, cx);
        }
        cx.notify();
    }

    fn toggle_list(&mut self, cx: &mut Context<Self>) {
        self.list_hidden = !self.list_hidden;
        let hidden = self.list_hidden;
        self.app
            .update(cx, |app, cx| {
                app.settings.saved.notes_list_hidden = hidden.then_some(true);
                app.save_settings_soon(cx);
            })
            .ok();
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        let app = self.app.clone();
        // The app takes the page away, which updates this one: after this update.
        cx.defer(move |cx| {
            app.update(cx, |app, cx| {
                app.close_notes(cx);
            })
            .ok();
        });
    }

    /// What was said into this note (voice/notes.rs), typed in at the
    /// editor's cursor, over a selection, spaced from the text either side,
    /// through the editor's own edit so it saves and undoes as typing does.
    /// Nothing if the note isn't the one showing any more.
    pub(crate) fn insert_dictated(&mut self, note: &str, words: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.selection.as_deref() != Some(note) {
            return;
        }
        let editor = self.editor.read(cx);
        let range = editor.selected_range();
        let text = editor.value().to_string();
        let (start, end) = (range.start.min(text.len()), range.end.min(text.len()));
        let before = text.get(..start).and_then(|t| t.chars().next_back());
        let after = text.get(end..).and_then(|t| t.chars().next());
        let Some(typed) = threadbear_core::voice::insertion(words, before, after) else { return };
        self.editor.update(cx, |e, cx| e.replace(typed, window, cx));
        self.body_typed(cx);
    }

    /// The microphone beside the bin, while Voice and Use with Notes are on:
    /// red with bars while it listens to this note.
    fn note_mic(&self, note: &str, cx: &mut Context<Self>) -> Option<AnyElement> {
        let voice = crate::voice::get(&self.app, cx);
        if !voice.read(cx).notes_available(cx) {
            return None;
        }
        let page = cx.entity().downgrade();
        Some(crate::voice::button::note_mic(&voice, note, page, cx))
    }

    // The box's typing help (`NoteEditing`).

    /// Return on a list line starts the next item, and on an empty item ends
    /// the list; anywhere else the editor's own Return goes ahead.
    fn on_enter(&mut self, action: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor.read(cx);
        let range = editor.selected_range();
        if action.shift || action.secondary || !range.is_empty() {
            cx.propagate();
            return;
        }
        let text = editor.value().to_string();
        let caret = range.start.min(text.len());
        let start = text[..caret].rfind('\n').map_or(0, |i| i + 1);
        let end = text[caret..].find('\n').map_or(text.len(), |i| caret + i);
        match notes::newline(&text[start..caret], &text[caret..end]) {
            notes::Newline::Plain => {
                cx.propagate();
                return;
            }
            notes::Newline::ContinueList(prefix) => self.editor.update(cx, |e, cx| e.replace(format!("\n{prefix}"), window, cx)),
            notes::Newline::EndList => self.editor.update(cx, |e, cx| {
                e.set_selected_range(start..caret, cx);
                e.replace("", window, cx);
            }),
        }
        cx.stop_propagation();
        self.body_typed(cx);
    }

    /// A click on a task's box ticks it, through the editor's own edit so
    /// ⌘Z unticks it, and the caret stays where it was.
    fn on_press(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let editor = self.editor.read(cx);
        let at = editor.offset_for_point(position);
        let text = editor.value().to_string();
        let Some(boxed) = notes::spans(&text).into_iter().find(|s| s.kind == Kind::Checkbox && s.range.start <= at && at < s.range.end) else { return false };
        self.tick(boxed.range.start, window, cx)
    }

    fn tick(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let text = self.editor.read(cx).value().to_string();
        let Some((range, mark)) = notes::toggled_box(&text, at) else { return false };
        self.editor.update(cx, |e, cx| {
            let was = e.selected_range();
            e.set_selected_range(range, cx);
            e.replace(mark, window, cx);
            e.set_selected_range(was, cx);
        });
        self.body_typed(cx);
        true
    }

    // Drawing.

    fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        let secondary = style::secondary_text(cx);
        let hover = style::primary_opacity(cx, 0.08);
        let fold_tip = if self.list_hidden { "Show the list of notes" } else { "Hide the list of notes" };
        let fold = tipped(div().id("notes-fold").size(px(26.)).rounded(px(6.)).flex().items_center().justify_center().cursor_pointer().hover(move |s| s.bg(hover)), fold_tip)
            .text_color(if self.list_hidden { secondary } else { cx.theme().foreground })
            .on_click(cx.listener(|this, _, _, cx| this.toggle_list(cx)))
            .child(icons::symbol("sidebar.left").with_size(px(14.)));
        let lock = tipped_as(div().id("notes-lock").flex_none(), "Encrypted on this Mac and never sent anywhere", gpui_kit::Role::Image)
            .text_color(style::tertiary_label(cx))
            .child(icons::symbol("lock.fill").with_size(px(10.)));
        let me = cx.entity().downgrade();
        let new = crate::push_button::PushButton::new("notes-new", "New Note", "Start a new note").disabled(self.store.problem.is_some()).render(cx, move |_, window, cx| {
            me.update(cx, |this, cx| this.new_note(window, cx)).ok();
        });
        let close_fill = style::primary_opacity(cx, 0.08);
        let close_hover = style::primary_opacity(cx, 0.14);
        let close = tipped(div().id("notes-close").size(px(28.)).rounded_full().bg(close_fill).hover(move |s| s.bg(close_hover)).flex().items_center().justify_center().cursor_pointer(), "Close Notes (Esc)")
            .on_click(cx.listener(|this, _, _, cx| this.close(cx)))
            .child(icons::symbol("xmark").with_size(px(12.)));
        h_flex()
            .flex_none()
            .gap(px(10.))
            .pl(px(20.))
            .pr(px(28.))
            .py(px(10.))
            .items_center()
            .border_b(gpui_kit::px(0.5))
            .border_color(style::hairline(cx))
            .child(fold)
            .child(div().text_size(px(17.)).font_weight(FontWeight::SEMIBOLD).child("Notes"))
            .child(lock)
            .child(div().flex_1())
            .child(new)
            .child(close)
            .into_any_element()
    }

    fn list(&self, cx: &mut Context<Self>) -> AnyElement {
        let selected_fill = style::primary_opacity(cx, 0.09);
        let hover = style::primary_opacity(cx, 0.05);
        let secondary = style::secondary_text(cx);
        let mut rows: Vec<AnyElement> = vec![];
        for note in self.store.sorted() {
            let id = note.id.clone();
            let selected = self.selection.as_deref() == Some(id.as_str());
            let (pick, rename, delete) = (id.clone(), id.clone(), id.clone());
            let me = cx.entity().downgrade();
            let row = div()
                .id(SharedString::from(format!("note-row-{id}")))
                .test_support()
                .role(gpui_kit::Role::ListItem)
                .aria_selected(selected)
                .aria_label(SharedString::from(note.title.clone()))
                .w_full()
                .px(px(10.))
                .py(px(7.))
                .rounded(px(7.))
                .cursor_pointer()
                .when(selected, |d| d.bg(selected_fill))
                .when(!selected, |d| d.hover(move |s| s.bg(hover)))
                .tip(note.title.clone())
                .on_click(cx.listener(move |this, _, window, cx| this.pick(Some(pick.clone()), window, cx)))
                .child(div().text_size(px(style::size::LABEL)).font_weight(FontWeight::MEDIUM).truncate().child(note.title.clone()))
                .child(div().text_size(px(style::size::SMALL)).text_color(secondary).truncate().child(note.preview()))
                .context_menu(move |menu, _, _| {
                    let (r, d, me_r, me_d) = (rename.clone(), delete.clone(), me.clone(), me.clone());
                    menu.item(PopupMenuItem::new("Rename").icon(icons::symbol("pencil")).on_click(move |_, window, cx| {
                        me_r.update(cx, |this, cx| {
                            this.pick(Some(r.clone()), window, cx);
                            this.title.update(cx, |t, cx| t.focus(window, cx));
                            crate::web_panels::keyboard_to_app(window);
                        })
                        .ok();
                    }))
                    .item(PopupMenuItem::new("Delete").icon(icons::symbol("trash")).on_click(move |_, window, cx| {
                        me_d.update(cx, |this, cx| this.ask_delete(d.clone(), window, cx)).ok();
                    }))
                });
            rows.push(row.into_any_element());
        }
        if self.store.notes.is_empty() {
            rows.push(div().p(px(14.)).text_size(px(style::size::LABEL)).text_color(secondary).child("No notes yet. New Note starts one.").into_any_element());
        }
        div()
            .id("notes-list")
            .flex_none()
            .w(px(LIST_WIDTH))
            .h_full()
            .border_r(gpui_kit::px(0.5))
            .border_color(style::hairline(cx))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(v_flex().p(px(8.)).gap(px(2.)).children(rows))
            .into_any_element()
    }

    fn note(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let secondary = style::secondary_text(cx);
        let Some(note) = self.open_note() else {
            let me = cx.entity().downgrade();
            let new = crate::push_button::PushButton::new("notes-new-empty", "New Note", "Start a new note").render(cx, move |_, window, cx| {
                me.update(cx, |this, cx| this.new_note(window, cx)).ok();
            });
            return v_flex()
                .flex_1()
                .h_full()
                .items_center()
                .justify_center()
                .gap(px(10.))
                .child(div().text_color(style::tertiary_label(cx)).child(icons::symbol("note.text").with_size(px(28.))))
                .child(div().text_color(secondary).child("Pick a note, or start a new one."))
                .child(new)
                .into_any_element();
        };
        let id = note.id.clone();
        let changed = notes::date_label(note.updated_at, true);
        let mic = self.note_mic(&id, cx);
        let caption = crate::voice::button::note_caption(&id, cx);
        let hover = style::primary_opacity(cx, 0.08);
        let bin = tipped(div().id("notes-delete").size(px(24.)).rounded(px(5.)).flex().items_center().justify_center().cursor_pointer().text_color(secondary).hover(move |s| s.bg(hover)), "Delete this note")
            .on_click(cx.listener(move |this, _, window, cx| this.ask_delete(id.clone(), window, cx)))
            .child(icons::symbol("trash").with_size(px(13.)));
        // A holder round the kit's field, named for scripts and VoiceOver.
        let title = tipped_as(div().id("notes-title").flex_1().min_w_0(), "The note's name. Click to rename it", gpui_kit::Role::GenericContainer)
            .child(Input::new(&self.title).appearance(false).bordered(false).text_size(px(16.)).font_weight(FontWeight::SEMIBOLD));
        let header = h_flex()
            .flex_none()
            .gap(px(8.))
            .px(px(16.))
            .py(px(10.))
            .items_center()
            .border_b(gpui_kit::px(0.5))
            .border_color(style::hairline(cx))
            .child(title)
            .child(div().flex_none().text_size(px(style::size::SMALL)).text_color(style::tertiary_label(cx)).whitespace_nowrap().child(changed))
            .children(mic)
            .child(bin);
        let editor = Editor::new(&self.editor)
            .h_full()
            .appearance(false)
            .bordered(false)
            .text_size(px(style::size::CODE))
            .line_height(px(style::size::CODE * 1.25 + 3.))
            .font_family(cx.theme().mono_font_family.clone());
        let me = cx.entity().downgrade();
        let body = div()
            .id("notes-editor")
            .flex_1()
            .min_h_0()
            .pl(px(4.))
            .pt(px(6.))
            .capture_action(cx.listener(|this, action: &Enter, window, cx| this.on_enter(action, window, cx)))
            .capture_any_mouse_down(move |event, window, cx| {
                if event.button != MouseButton::Left || event.click_count != 1 {
                    return;
                }
                if me.update(cx, |this, cx| this.on_press(event.position, window, cx)).unwrap_or(false) {
                    cx.stop_propagation();
                    window.prevent_default();
                }
            })
            .child(editor);
        let _ = window;
        v_flex().flex_1().min_w_0().h_full().child(header).child(body).children(caption).into_any_element()
    }
}

impl Render for NotesPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The highlighter's colours, from the theme as it's drawn now. Made
        // solid over the page: the editor blends a run's colour over its own
        // text colour, so a see-through grey over white came out white.
        let ground = cx.theme().background;
        *self.palette.borrow_mut() = Palette {
            accent: style::control_accent(cx),
            secondary: ground.blend(style::secondary_label(cx)),
            tertiary: ground.blend(style::tertiary_label(cx)),
            code: if cx.theme().is_dark() { gpui_kit::rgb(0xff375f).into() } else { gpui_kit::rgb(0xff2d55).into() },
            code_background: style::inline_code_background(cx),
            link: style::link_text(cx),
        };
        let body: AnyElement = match &self.store.problem {
            Some(problem) => div().flex_1().flex().items_center().justify_center().text_color(style::secondary_text(cx)).child(problem.clone()).into_any_element(),
            None => h_flex().flex_1().min_h_0().overflow_hidden().when(!self.list_hidden, |d| d.child(self.list(cx))).child(self.note(window, cx)).into_any_element(),
        };
        v_flex()
            .id("notes-page")
            .key_context(PAGE_CONTEXT)
            .track_focus(&self.focus)
            .size_full()
            .bg(cx.theme().background)
            .on_action(cx.listener(|this, _: &CloseNotes, _, cx| this.close(cx)))
            .child(self.header(cx))
            .child(body)
    }
}

/// The editor's colouring: core's spans (`NoteHighlight`), worked out again
/// on each change (a note is small), painted in runs that cover what's asked.
struct NoteHighlighter {
    spans: Vec<notes::Span>,
    palette: Rc<RefCell<Palette>>,
}

impl NoteHighlighter {
    fn style(&self, span: &notes::Span) -> HighlightStyle {
        let p = *self.palette.borrow();
        let bold = Some(FontWeight::BOLD);
        match span.kind {
            // The Swift app sizes headings by level; GPUI's runs can't, so they're bold.
            Kind::Heading => HighlightStyle { font_weight: bold, ..Default::default() },
            Kind::Checkbox | Kind::ListMarker => HighlightStyle { font_weight: bold, color: Some(p.accent), ..Default::default() },
            Kind::Done => HighlightStyle { color: Some(p.secondary), strikethrough: Some(StrikethroughStyle { thickness: gpui_kit::px(1.), color: Some(p.secondary) }), ..Default::default() },
            Kind::Bold => HighlightStyle { font_weight: bold, ..Default::default() },
            Kind::Italic => HighlightStyle { font_style: Some(FontStyle::Italic), ..Default::default() },
            Kind::Code | Kind::CodeBlock => HighlightStyle { color: Some(p.code), background_color: Some(p.code_background), ..Default::default() },
            Kind::Link => HighlightStyle { color: Some(p.link), underline: Some(UnderlineStyle { thickness: gpui_kit::px(1.), color: Some(p.link), wavy: false }), ..Default::default() },
            Kind::Quote => HighlightStyle { color: Some(p.secondary), ..Default::default() },
            Kind::Mark => HighlightStyle { color: Some(p.tertiary), ..Default::default() },
        }
    }
}

impl InputHighlighter for NoteHighlighter {
    fn language(&self) -> SharedString {
        NOTE_LANGUAGE.into()
    }

    fn update(&mut self, _edit: Option<InputEdit>, text: &Rope, _folding: bool, _window: &mut Window, _cx: &mut Context<EditorState>) {
        self.spans = notes::spans(&text.to_string());
    }

    fn styles(&self, range: &Range<usize>, _resolver: &dyn HighlightStyleResolver) -> Vec<(Range<usize>, HighlightStyle)> {
        let hits: Vec<&notes::Span> = self.spans.iter().filter(|s| s.range.start < range.end && s.range.end > range.start).collect();
        let mut cuts: Vec<usize> = vec![range.start, range.end];
        for s in &hits {
            cuts.push(s.range.start.clamp(range.start, range.end));
            cuts.push(s.range.end.clamp(range.start, range.end));
        }
        cuts.sort_unstable();
        cuts.dedup();
        let mut out: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            // Later spans over earlier ones, as the Swift app adds attributes.
            let style = hits.iter().filter(|s| s.range.start <= a && s.range.end >= b).fold(HighlightStyle::default(), |acc, s| acc.highlight(self.style(s)));
            match out.last_mut() {
                Some((last, prev)) if *prev == style && last.end == a => last.end = b,
                _ => out.push((a..b, style)),
            }
        }
        out
    }

    fn fold_ranges(&self, _text: &Rope) -> Vec<FoldRange> {
        vec![]
    }
}

/// The automation's `notes` command (features/notes.feature): Notes opened,
/// or with `rename` the open note renamed through its title field, `pick` a
/// note picked from the list, `tick` the box on that line clicked, and
/// `confirm` the delete that's asking answered Delete.
#[cfg(feature = "automation")]
pub(crate) fn automation(app: &Entity<Threadbear>, cmd: &Value, window: &mut Window, cx: &mut App) -> Result<Value, String> {
    use gpui_kit::test::{TestEventExt as _, TestWindowExt as _};
    if cmd["close"].as_bool() == Some(true) {
        app.update(cx, |t, cx| t.close_notes(cx));
        return Ok(json!("ok closed"));
    }
    if app.read(cx).notes_page.is_none() {
        app.update(cx, |t, cx| t.open_notes(window, cx));
    }
    let Some(page) = app.read(cx).notes_page.clone() else { return Err("Show Notes is off".into()) };
    if let Some(title) = cmd["rename"].as_str() {
        page.update(cx, |p, cx| {
            p.title.update(cx, |t, cx| t.set_value(title.to_owned(), window, cx));
            p.commit_title(cx);
            p.show_title(window, cx);
        });
    }
    if let Some(title) = cmd["pick"].as_str() {
        let id = page.read(cx).store.notes.iter().find(|n| n.title == title).map(|n| n.id.clone()).ok_or_else(|| format!("no note called {title:?}"))?;
        page.update(cx, |p, cx| p.pick(Some(id), window, cx));
    }
    if let Some(line) = cmd["tick"].as_u64() {
        // Through a real click on the box, where it's drawn.
        window.render_frame(cx);
        let text = page.read(cx).editor.read(cx).value().to_string();
        let start: usize = text.split('\n').take(line.saturating_sub(1) as usize).map(|l| l.len() + 1).sum();
        let at = notes::spans(&text).into_iter().find(|s| s.kind == Kind::Checkbox && s.range.start >= start && !text[start..s.range.start].contains('\n')).map(|s| s.range.start).ok_or_else(|| format!("no box on line {line}"))?;
        let bounds = page.read(cx).editor.read(cx).range_to_bounds(&(at + 1..at + 2)).ok_or("the box isn't on screen")?;
        let point = bounds.center();
        window.pointer_move(point, None, cx);
        window.pointer_down(point, MouseButton::Left, cx);
        window.pointer_up(point, MouseButton::Left, cx);
        window.render_frame(cx);
    }
    if cmd["confirm"].as_bool() == Some(true) {
        let title = page.read(cx).deleting.clone().ok_or("no delete is asking")?;
        let id = page.read(cx).store.notes.iter().find(|n| n.title == title).map(|n| n.id.clone()).ok_or("the note has gone")?;
        page.update(cx, |p, cx| p.delete(&id, window, cx));
        window.close_dialog(cx);
    }
    Ok(json!("ok"))
}
