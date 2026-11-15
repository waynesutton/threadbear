// A text file worked on in the pane, a port of the editing half of the Swift
// app's TextFileView: the bar's pencil (or a Markdown file's Markdown view)
// puts up the file's own text with line numbers, ready to type in. What's
// typed is saved into the file a moment after the typing stops, on ⌘S, on
// Done, on leaving the file and on quitting. The file is watched, so an
// agent's rewrite shows up by itself; if it lands under unsaved typing, a bar
// asks which to keep rather than losing either (`ConflictBar`).
//
// The rules are threadbear_core::text_file::EditSession's, which reads and
// writes the file on this machine, in place, as the Swift app does: only a
// thread on this runner gets a pencil, as another Mac's file would only be a
// copy here. Nothing is written but the file showing, and only for Mike's
// typing.
use crate::file_watch::FolderWatch;
use crate::highlights::{icon, palette, SidePane};
use crate::push_button::PushButton;
use crate::style::{self, px};
use gpui_kit::component::input::{Editor, EditorState, InputEvent};
use gpui_kit::component::{h_flex, v_flex, ActiveTheme as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use threadbear_core::text_file::{words, EditSession, Saved, CHECK_DELAY_MS, EDITOR_SAVE_DELAY_MS, SAVE_DELAY_MS};

actions!(file_preview, [SaveFile]);

/// How long the bar's quiet "✓ Saved" stays after a save lands.
const JUST_SAVED_MS: u64 = 1500;

/// What leaving a file with changes not saved does, when Save files
/// automatically is off: Save, Don't Save or Cancel (`EditedFiles.Answer`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LeaveAnswer {
    Save,
    Discard,
    Cancel,
}

impl LeaveAnswer {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "save" => Some(LeaveAnswer::Save),
            "discard" => Some(LeaveAnswer::Discard),
            "cancel" => Some(LeaveAnswer::Cancel),
            _ => None,
        }
    }
}

/// ⌘S, bound once for every window, in the file's key context.
pub(crate) fn bind_keys(cx: &mut App) {
    static BOUND: std::sync::Once = std::sync::Once::new();
    let chord = if cfg!(target_os = "macos") { "cmd-s" } else { "ctrl-s" };
    BOUND.call_once(|| cx.bind_keys([KeyBinding::new(chord, SaveFile, Some(crate::file_find::CONTEXT))]));
}

/// The file's text in an editor of its own, and the document it came from.
pub(crate) struct FileEdit {
    pub(crate) session: EditSession,
    pub(crate) editor: Entity<EditorState>,
    /// The save a moment after the last keystroke.
    save_soon: Option<Task<()>>,
    /// A read of the file once a burst of changes to it settles.
    check_soon: Option<Task<()>>,
    /// The file's folder, watched for the file changing under the editor.
    _watch: Option<FolderWatch>,
    _typing: Subscription,
    /// The cursor moving, for the line and column in the bar under the text.
    _cursor: Subscription,
    /// Edit files like an editor, as it was when the file opened: the bar
    /// under the text, and saves that wait for ⌘S while autosave is off.
    pub(crate) editor_mode: bool,
    /// Typed and not in the file yet: the bar says Edited and the file's
    /// name has a dot.
    pub(crate) dirty: bool,
    /// A save just landed: the bar's quiet "✓ Saved", for a moment.
    pub(crate) just_saved: bool,
    _just_saved_clear: Option<Task<()>>,
}

impl FileEdit {
    /// Reads `path` here and puts its text in a new editor: coloured as code
    /// when it's code (`language`), with line numbers down its side.
    pub(crate) fn open(thread_id: &str, path: &str, language: Option<&'static str>, editor_mode: bool, window: &mut Window, cx: &mut Context<SidePane>) -> Self {
        let session = EditSession::open(path, true);
        let text = session.view_text(session.document.saved());
        let editor = cx.new(|cx| {
            // As AppKit's text view behaves: a tab is a tab, Return starts a
            // plain new line, and nothing is folded, guided or closed for you.
            let state = EditorState::new(window, cx)
                .soft_wrap(true)
                .line_number(true)
                .folding(false)
                .indent_guides(false)
                .auto_close(false)
                .smart_indent(false)
                .searchable(false)
                // The find bar's Replace and All (file_find.rs).
                .replaceable(true)
                .tab_size(gpui_kit::component::input::TabSize { tab_size: 4, hard_tabs: true });
            match language {
                Some(language) => state.language(language),
                None => state,
            }
        });
        // Setting the text says nothing to the subscription below, which
        // hears only typing.
        editor.update(cx, |s, cx| s.set_value(text, window, cx));
        let id = thread_id.to_owned();
        let typing = cx.subscribe(&editor, move |this: &mut SidePane, _, ev: &InputEvent, cx| {
            if matches!(ev, InputEvent::Change) {
                this.edit_typed(&id, cx);
            }
        });
        // The file's own folder: an agent's write renames a new file over
        // this one, which a watch on the file itself would lose.
        let file = Path::new(path).file_name().map(|n| n.to_owned());
        let folder = Path::new(path).parent().map(|p| p.to_string_lossy().into_owned());
        let id = thread_id.to_owned();
        let watch = folder.and_then(|folder| {
            let mut watch = FolderWatch::new(cx, move |this: &mut SidePane, _, paths, cx| {
                if paths.iter().any(|p| p.file_name() == file.as_deref()) {
                    this.edit_touched(&id, cx);
                }
            })?;
            watch.set(&HashSet::from([folder]));
            Some(watch)
        });
        let cursor = cx.observe(&editor, |_, _, cx| cx.notify());
        FileEdit { session, editor, save_soon: None, check_soon: None, _watch: watch, _typing: typing, _cursor: cursor, editor_mode, dirty: false, just_saved: false, _just_saved_clear: None }
    }

    /// Where the cursor is, from 1, for the bar under the text.
    pub(crate) fn cursor(&self, cx: &App) -> (u32, u32) {
        let at = self.editor.read(cx).cursor_position();
        (at.line + 1, at.character + 1)
    }

    /// Whether what's typed is written by itself a moment after the typing
    /// stops: always outside editor mode, and in it while Save files
    /// automatically is on (`autosaves`).
    pub(crate) fn autosaves(&self, autosave: bool) -> bool {
        !self.editor_mode || autosave
    }

    /// The bar's words for whether what's typed is in the file yet.
    pub(crate) fn state_words(&self, autosave: bool) -> &'static str {
        if self.dirty {
            words::edited(self.autosaves(autosave))
        } else {
            "Saved"
        }
    }

    /// Whether the editor holds text the file doesn't.
    fn differs(&self, cx: &App) -> bool {
        self.session.takes_typing() && *self.editor.read(cx).value() != *self.session.document.saved()
    }
}

impl SidePane {
    fn edit_mut(&mut self, thread_id: &str) -> Option<&mut FileEdit> {
        self.files.open.get_mut(thread_id)?.edit.as_mut()
    }

    /// Settings > General > Files: (Edit files like an editor, Save files
    /// automatically).
    pub(crate) fn file_modes(&self, cx: &App) -> (bool, bool) {
        self.app.upgrade().map_or((true, true), |app| {
            let s = &app.read(cx).settings;
            (s.edits_files_like_an_editor(), s.saves_files_automatically())
        })
    }

    /// Whether the thread's file has typing in it that isn't saved yet.
    pub(crate) fn file_unsaved(&self, thread_id: &str) -> bool {
        self.files.open.get(thread_id).and_then(|o| o.edit.as_ref()).is_some_and(|e| e.dirty)
    }

    /// Something was typed: saved a moment after the typing stops while it
    /// autosaves, else marked Edited until ⌘S (`textDidChange`).
    pub(crate) fn edit_typed(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let autosave = self.file_modes(cx).1;
        let Some(edit) = self.edit_mut(thread_id) else { return };
        if !edit.session.takes_typing() {
            return;
        }
        edit.session.typed();
        edit.dirty = edit.differs(cx);
        cx.notify();
        if !edit.dirty || !edit.autosaves(autosave) {
            edit.save_soon = None;
            return;
        }
        let delay = if edit.editor_mode { EDITOR_SAVE_DELAY_MS } else { SAVE_DELAY_MS };
        let id = thread_id.to_owned();
        edit.save_soon = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(delay)).await;
            this.update(cx, |this, cx| {
                this.save_edit(&id, None, cx);
            })
            .ok();
        }));
    }

    /// Writes what's typed now (`saveNow`): ⌘S, the pause after typing,
    /// Done, leaving the file, quitting. What it came to.
    pub(crate) fn save_edit(&mut self, thread_id: &str, window: Option<&mut Window>, cx: &mut Context<Self>) -> Saved {
        let handle = self.files.window;
        let Some(edit) = self.files.open.get_mut(thread_id).and_then(|o| o.edit.as_mut()) else { return Saved::Done };
        edit.save_soon = None;
        let was_dirty = edit.dirty;
        let typed = edit.editor.read(cx).value().to_string();
        let saved = edit.session.save_now(&typed);
        if !matches!(saved, Saved::Kept) {
            edit.dirty = false;
            // The quiet tick, only when something was written.
            if was_dirty {
                edit.just_saved = true;
                let id = thread_id.to_owned();
                edit._just_saved_clear = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(Duration::from_millis(JUST_SAVED_MS)).await;
                    this.update(cx, |this, cx| {
                        if let Some(edit) = this.edit_mut(&id) {
                            edit.just_saved = false;
                            cx.notify();
                        }
                    })
                    .ok();
                }));
            }
        }
        if let Saved::DoneShowing(text) = &saved {
            let editor = edit.editor.clone();
            show_text(&editor, text.clone(), window, handle, cx);
        }
        cx.notify();
        saved
    }

    /// The file changed on disk: read once its burst of changes settles.
    fn edit_touched(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let Some(edit) = self.edit_mut(thread_id) else { return };
        let id = thread_id.to_owned();
        edit.check_soon = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(CHECK_DELAY_MS)).await;
            this.update(cx, |this, cx| this.check_edit(&id, cx)).ok();
        }));
    }

    /// What the change means for what's on screen (`fileChanged`): the file's
    /// new text when nothing's typed, else the conflict bar.
    pub(crate) fn check_edit(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let handle = self.files.window;
        let Some(edit) = self.edit_mut(thread_id) else { return };
        edit.check_soon = None;
        let in_view = edit.editor.read(cx).value().to_string();
        let shown = edit.session.shown(&in_view).to_owned();
        let change = edit.session.document.check(&shown);
        if let Some(text) = edit.session.file_changed(change) {
            // The file's text shows: nothing of Mike's is waiting.
            edit.dirty = false;
            let editor = edit.editor.clone();
            show_text(&editor, text, None, handle, cx);
        }
        cx.notify();
    }

    /// The conflict bar's "Take the file's": what's on disk shows, what was
    /// typed since the last save goes (`useFile`).
    pub(crate) fn take_files_side(&mut self, thread_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.edit_mut(thread_id) else { return };
        let Some(file_text) = edit.session.conflict.clone() else { return };
        let text = edit.session.use_file(file_text);
        edit.dirty = false;
        let editor = edit.editor.clone();
        show_text(&editor, text, Some(window), None, cx);
        cx.notify();
    }

    /// Its "Keep mine": what's typed is written over the file (`keepMine`).
    pub(crate) fn keep_mine(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let Some(edit) = self.edit_mut(thread_id) else { return };
        let typed = edit.editor.read(cx).value().to_string();
        if !matches!(edit.session.keep_mine(&typed), Saved::Kept) {
            edit.dirty = false;
        }
        cx.notify();
    }

    /// Whether the thread's file can be left now (`mayLeaveFile`): nothing
    /// unsaved, or saved now while it autosaves. With Save files
    /// automatically off and changes typed it asks Save, Don't Save or Cancel
    /// and says no; `then` does the leaving once Save or Don't Save is picked.
    pub(crate) fn may_leave_file(&mut self, thread_id: &str, then: impl FnOnce(&mut Self, &mut Context<Self>) + 'static, cx: &mut Context<Self>) -> bool {
        let autosave = self.file_modes(cx).1;
        let Some(edit) = self.edit_mut(thread_id) else { return true };
        if !edit.dirty {
            return true;
        }
        if edit.autosaves(autosave) {
            self.save_edit(thread_id, None, cx);
            return true;
        }
        let name = threadbear_core::highlights::last_component(edit.session.document.path().to_string_lossy().as_ref()).to_owned();
        // A script's answer, given before (a scripted app can't click an alert).
        if let Some(answer) = self.files.leave_answer.take() {
            return self.leave_answered(thread_id, answer, cx);
        }
        if cfg!(feature = "automation") {
            return false;
        }
        let Some(handle) = self.files.window else {
            // Nowhere to ask: keeping what's typed beats losing it.
            self.save_edit(thread_id, None, cx);
            return true;
        };
        let (id, side) = (thread_id.to_owned(), cx.entity().downgrade());
        // Asked once the click that led here has finished.
        cx.defer(move |cx| {
            let answers = [PromptButton::new("Save"), PromptButton::cancel("Cancel"), PromptButton::new("Don't Save")];
            let title = format!("Save the changes to {name}?");
            let Ok(asked) = handle.update(cx, |_, window, cx| window.prompt(PromptLevel::Warning, &title, Some("If you don't save them, what you typed is lost."), &answers, cx)) else { return };
            cx.spawn(async move |cx| {
                let answer = match asked.await {
                    Ok(0) => LeaveAnswer::Save,
                    Ok(2) => LeaveAnswer::Discard,
                    _ => return,
                };
                side.update(cx, |this, cx| {
                    if this.leave_answered(&id, answer, cx) {
                        then(this, cx);
                    }
                })
                .ok();
            })
            .detach();
        });
        false
    }

    /// What Save, Don't Save or Cancel came to: whether the file can go.
    /// Save that can't write (a conflict waiting) stays, for the bar.
    fn leave_answered(&mut self, thread_id: &str, answer: LeaveAnswer, cx: &mut Context<Self>) -> bool {
        match answer {
            LeaveAnswer::Cancel => false,
            LeaveAnswer::Save => !matches!(self.save_edit(thread_id, None, cx), Saved::Kept),
            // What's typed goes, and nothing is written on the way out: the
            // file's text is read again if it's shown again.
            LeaveAnswer::Discard => {
                if let Some(open) = self.files.open.get_mut(thread_id) {
                    open.edit = None;
                }
                cx.notify();
                true
            }
        }
    }

    /// The thread's file is leaving the pane (Back, another file, another
    /// section or thread): what's typed is saved first (`stop`).
    pub(crate) fn leave_edit(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        if self.files.open.get(thread_id).is_some_and(|o| o.edit.as_ref().is_some_and(|e| e.session.drafted())) {
            self.save_edit(thread_id, None, cx);
        }
    }

    /// Quitting mid-edit: every file being worked on is saved before the app
    /// goes (`willTerminateNotification`).
    pub(crate) fn save_every_edit(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<String> = self.files.open.iter().filter(|(_, o)| o.edit.as_ref().is_some_and(|e| e.session.drafted())).map(|(id, _)| id.clone()).collect();
        for id in ids {
            self.save_edit(&id, None, cx);
        }
    }

    /// The raw text with what's over it: the conflict bar, or the grey note
    /// (cut off, can't be edited, gone), then the find bar.
    pub(crate) fn render_edit(&self, thread_id: &str, edit: &FileEdit, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let bar = if edit.session.conflict.is_some() {
            Some(self.conflict_bar(thread_id, cx))
        } else {
            edit.session.note.clone().map(|text| {
                // `NoteBar`: an ⓘ and the words, grey, over a hairline.
                h_flex()
                    .flex_none()
                    .w_full()
                    .px(px(14.))
                    .py(px(7.))
                    .gap(px(8.))
                    .items_center()
                    .border_b_1()
                    .border_color(p.hairline)
                    .text_size(px(style::size::LABEL))
                    .line_height(px(style::natural_line(style::size::LABEL)))
                    .text_color(p.secondary)
                    .child(icon("info.circle", style::size::LABEL, p.secondary).flex_none())
                    .child(div().flex_1().min_w_0().line_clamp(2).child(text))
                    .into_any_element()
            })
        };
        let find = self.render_find_bar(thread_id, cx);
        let editor = Editor::new(&edit.editor)
            // Shown, but never written back, when the file can't take it
            // (too big, not UTF-8): it would change what's there.
            .readonly(!edit.session.takes_typing())
            .h_full()
            .appearance(false)
            .bordered(false)
            .text_size(px(style::size::DETAIL))
            .line_height(px(crate::files::CODE_LINE))
            .font_family(cx.theme().mono_font_family.clone());
        let status = (edit.editor_mode && edit.session.takes_typing()).then(|| self.status_bar(thread_id, edit, cx));
        // The text starts where the Swift app's does, past its 44 point
        // gutter of line numbers.
        v_flex().size_full().children(bar).children(find).child(div().flex_1().min_h_0().pl(px(14.)).child(editor)).children(status).into_any_element()
    }

    /// `EditorStatusBar`, under the text in Edit files like an editor: where
    /// the cursor is, whether what's typed is in the file yet, and Save.
    fn status_bar(&self, thread_id: &str, edit: &FileEdit, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let autosave = self.file_modes(cx).1;
        let (line, column) = edit.cursor(cx);
        let (id, side) = (thread_id.to_owned(), cx.entity().downgrade());
        let save = PushButton::new("file-save", words::SAVE, words::SAVE_TIP).small().disabled(!edit.dirty || edit.session.conflict.is_some()).render(cx, move |_, window, cx| {
            side.update(cx, |this, cx| {
                this.save_edit(&id, Some(window), cx);
            })
            .ok();
        });
        let state: AnyElement = if edit.dirty {
            h_flex().gap(px(5.)).items_center().child(div().size(px(6.)).rounded_full().bg(p.accent)).child(edit.state_words(autosave)).into_any_element()
        } else if edit.just_saved {
            h_flex().gap(px(4.)).items_center().child(icon("checkmark", 10., p.secondary)).child("Saved").into_any_element()
        } else {
            div().text_color(p.faint).child("Saved").into_any_element()
        };
        h_flex()
            .id("file-status")
            .flex_none()
            .w_full()
            .px(px(12.))
            .py(px(4.))
            .gap(px(10.))
            .items_center()
            .border_t_1()
            .border_color(p.hairline)
            .text_size(px(style::size::LABEL))
            .line_height(px(style::natural_line(style::size::LABEL)))
            .text_color(p.secondary)
            .child(format!("Line {line}, Column {column}"))
            .child(div().flex_1())
            .child(state)
            .child(save)
            .into_any_element()
    }

    /// `ConflictBar`: the file changed on disk while the editor holds unsaved
    /// typing, and Mike picks a side. Saving waits until he does, so neither
    /// is written over. Two lines: the pane is narrow, and a row with the
    /// buttons on it cut the words and the buttons both short.
    fn conflict_bar(&self, thread_id: &str, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let (take_id, keep_id) = (thread_id.to_owned(), thread_id.to_owned());
        let side = cx.entity().downgrade();
        let side2 = side.clone();
        let take = PushButton::new("conflict-take", words::TAKE_FILES, words::TAKE_FILES_TIP).render(cx, move |_, window, cx| {
            side.update(cx, |this, cx| this.take_files_side(&take_id, window, cx)).ok();
        });
        let keep = PushButton::new("conflict-keep", words::KEEP_MINE, words::KEEP_MINE_TIP).render(cx, move |_, _, cx| {
            side2.update(cx, |this, cx| this.keep_mine(&keep_id, cx)).ok();
        });
        v_flex()
            .flex_none()
            .w_full()
            .px(px(14.))
            .py(px(9.))
            .gap(px(8.))
            .bg(style::warning_fill(cx))
            .text_size(px(style::size::LABEL))
            .line_height(px(style::natural_line(style::size::LABEL)))
            .text_color(p.fg)
            // The icon beside the two lines' middle, as the Swift bar's HStack has it.
            .child(h_flex().gap(px(8.)).items_center().child(icon("arrow.triangle.2.circlepath", 15., p.secondary).flex_none()).child(div().flex_1().min_w_0().child(words::CONFLICT)))
            .child(h_flex().gap(px(8.)).child(take).child(keep))
            .into_any_element()
    }
}

/// Puts the file's text in the editor, keeping the cursor's line where it
/// can (`show(_:keepPlace:)`): an agent's rewrite or the file's side shouldn't
/// throw Mike back to the top. The editor needs the window: the one at hand,
/// or the pane's (`handle`) for a save or a check that came later.
fn show_text(editor: &Entity<EditorState>, text: String, window: Option<&mut Window>, handle: Option<AnyWindowHandle>, cx: &mut App) {
    let put = |window: &mut Window, cx: &mut App| {
        editor.update(cx, |s, cx| {
            let mut place = s.cursor_position();
            // The cursor's line, if the new text still has it.
            place.line = place.line.min(text.matches('\n').count() as u32);
            s.set_value(text, window, cx);
            s.set_cursor_position(place, window, cx);
        });
    };
    match (window, handle) {
        (Some(window), _) => put(window, cx),
        (None, Some(handle)) => {
            let _ = handle.update(cx, |_, window, cx| put(window, cx));
        }
        (None, None) => {}
    }
}
